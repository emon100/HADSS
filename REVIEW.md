# HADSS 完整 Review

> 评审对象：`emon100/HADSS`，当前分支 `arena/01a07431-hadss`，基线 commit `7dae622`（2022-06-11，作者 emon100）。
> 评审方式：静态代码 + 文档审阅（沙箱无网络无法安装 Go/Rust，未能实际编译运行）。

---

## 1. 项目愿景（README 所述）

> HADSS = **High Availability Distributed Storage System**，可作为对象存储服务（Object Storage Service）的后端。

设计哲学（README「Design Philosophy」）：
1. **No Single Point Failure**（无单点故障）
2. **集群扩缩容时依旧响应灵敏**（Responsive when clusters scale up/down）
3. **即使整个 Index Layer 宕机，也能响应基本请求**

### 架构：三层
- **Gateway Layer**：面向用户，把不同存储服务包装成 RPC，承载对象存储的业务逻辑；生产环境前置负载均衡器。
- **Index Layer**：监控 Storage Layer 状态并执行恢复（监控扩缩容、健康检查、向其他层提供节点信息、检测并修复数据损坏、检测并恢复节点故障）。README 中以 `Monitor` 组件实现。
- **Storage Layer**：实际存储，接收 `(Handler, ConsistencyPolicy, NodeStatusVersion)`，通过 `NodeStatus` 自动均衡各存储节点的存储占用。

### README 中的关键工作流（sequence diagram 描述）
- 写入：Gateway 算 metadata → 通过**一致性哈希**定位节点 → 异步 PUT → 存储节点内部做**Quorum 写**（[Handler, ConsistencyPolicy] 拷贝到多个副本）。
- 读取：一致性哈希定位 → **Quorum 读**保证读到最新内容。
- 扩缩容：Index Layer 收集健康节点 → 计算新节点状态与数据分布 → 触发节点加入分组（grouping）与再平衡（reallocating），新旧段之间做数据迁移。

---

## 2. 代码结构盘点

| 目录 | 语言 | 类 型 | 说明 |
|---|---|---|---|
| `Gateway/` | Go + Gin | Gateway Layer | HTTP 网关，`GET/PUT /id/:id` |
| `Monitor/` | Go + Gin + etcd client | Index Layer | 心跳、节点状态收集、分区计算、分布式锁 |
| `Storage/` | Rust + actix-web + openraft | Storage Layer | 基于 Raft 的复制状态机 + sled 日志 + FS 文件 |
| `StorageConnector/` | Go | 公共客户端库 | 封装到 Storage 节点的 HTTP 连接 + 一致性策略 |
| `TLAspec/` | TLA+ | 形式化规范 | **空**，仅占位 |
| `e2e_tests/` | Node | 端到端示意 | 独立 mock，**未接入真实系统** |
| `run.sh` | Bash | 集成启动脚本 | 启动 etcd/gateway/monitor/storage |

---

## 3. 各层实际实施情况

### 3.1 Gateway Layer（Go + Gin）
已实现：
- `GET /id/:id`、`PUT /id/:id` 两个对象读写接口。
- SHA-256(`id`) 作为 key，通过 Nodemap（`NodesRanges` 记录「哈希区间 → 节点组」）做一致性哈希定位存储节点。
- 后台每 10s 从 `Monitor/nodemap` 拉取并缓存 Nodemap（最近提交对应的「cache nodemap」功能）。
- 写入/读取时按节点组逐个重试（对应 commit 标题「retry all Storage Nodes」）。

代码问题：
- **读取非 Quorum**：`getId` 只是依次尝试组内节点，第一个返回 200 即返回，不校验版本/多数派，与 README「Quorum read」不符。`get_slice` 的 `<id>` 直接落到本地 FS。
- **资源泄漏**：轮询 goroutine 内对每个成功响应 `defer g.Body.Close()`，defer 在 goroutine 函数结束时才执行，而非每轮迭代，连接会持续累积。
- **日志 bug**：`fmt.Println("%s", ...)` 用了 Println 但带格式串，会原样打印 `%s`。
- **一致性策略未贯通**：`connector.NewBasicConnection(addr, connector.StrongConsistency)` 传了策略，但存储层完全忽略该参数。
- **不支持流式**：`ioutil.ReadAll(c.Request.Body)` 全量读入内存，且 `ContentLength <= 0` 直接 502，不能处理流式/大文件。

### 3.2 Index Layer / Monitor（Go + etcd）
已实现：
- `POST /heartbeat`：把节点心跳写入 etcd `/nodes/<nodeid>`。
- `GET /nodemap`：从 etcd 读取并返回 Nodemap。
- `CollectNodesInfo`：遍历 etcd 前缀 `/nodes` 收集所有节点。
- `startRepeatingCalculation`：用 etcd 分布式锁（`concurrency.Mutex`）保证多 Monitor 中只有一个在算，定期（约 10s）重算 Nodemap 并写回 `/nodemap`。
- `tryCalculateNewNodemap`：把每 3 个 `ready` 节点凑成一个分组，并通过 HTTP 对首个节点执行 `init/add-learner/change-membership` 来初始化 Raft 集群。

代码问题（多为 README 中规划但未落地/有缺陷）：
- **心跳无 lease/TTL**：`heartbeat` 直接 `Put` 到 etcd，**没有设置过期时间**，死节点永远不会被清理 → 无法检测/移除节点故障。README 明确要求「使用 lease」，代码里只有 `//TODO`。
- **范围切分算法有 bug**：注释自曝「last one will not fill the whole area because 10/3*3 == 9 but 9 != 10」。且 `rangeSplit := newGroupCounts + len(oldNodemap.NodesRanges)` 在已经 `append` 新组后重复计数（double count）。
- **扩容逻辑只增不减**：没有缩容/移除节点、没有真正的数据迁移（reallocating）。README 描述的「扩容/缩容再平衡」基本未实现；`MockProcessNodesInfo` 只是注释掉的占位。
- **硬编码集群初始化**：`add-learner` 固定用 `[2, ...]`、`[3, ...]`、`change-membership [1,2,3]`，默认节点总是 3 个、固定地址，无法泛化。
- **与节点通信方式脆弱**：直接在计算循环里用 `http.Post` 控制存储节点，顺序强耦合、无错误处理（`//TODO new err handle`）。
- **Monitor 只读旧 nodemap**：`getNodemap` 只是读 etcd，没有任何"计算"逻辑（注释也说明待实现：由持有锁的 Monitor 计算）。

### 3.3 Storage Layer（Rust + openraft）
已实现：
- 基于 `openraft` 的 Raft 复制状态机；每个存储节点是一个 Raft 节点。
- 支持 `init / add-learner / change-membership / metrics` 管理接口，以及 `raft-vote / raft-append / raft-snapshot` RPC。
- 应用层接口 `GET/PUT /slice/<id>`（id 为 `64hex.<对象名>`）。
- 日志用 sled 树存储，state machine（`last_applied_log`、`last_membership`、`data: Vec<String>`、`nodemap_version`）在内存中用 `RwLock`（注：state machine 本身没有持久化到裸盘，仅靠 raft 快照）。
- `fs_io` 把文件按目录深度切分（如 `12/34/56/7a8b`）写入本地文件系统。

严重问题：
1. **快照不包含文件字节 —— 数据丢失隐患（最关键）**：
   `StorageNodeFileStoreStateMachine.data` 只保存文件 **id 字符串**，**文件内容字节由 `fs_io::store_slice` 写到磁盘**。而 `build_snapshot` 序列化的是 state machine（仅 id 列表），不包含磁盘上的文件内容；`install_snapshot` 恢复的也只是 id 列表。**一个通过快照追平日志的新节点/落后节点，会拿到一堆 id 却没有对应文件字节 → 数据不完整**。快照是 Raft 日志压缩的基石，这条路径损坏意味着无法可靠地通过快照进行节点恢复。
2. **`store_slice` 失败被吞掉**：`apply_to_state_machine` 里 `if let Err(_) = fs_io::store_slice(...)` 静默忽略写入错误（`//TODO: return error`），但 `sm.data.push(key)` 仍然执行 → state machine 记录了并未真正落盘的文件，造成状态与实际不一致。
3. **一致性策略完全被忽略**：`slice.rs` 的 `get_slice`/`put_slice` 从不读取 `consistency_policy` 查询参数；且 reads 走的是**本地 FS 直读**，不走 Raft、不校验日志版本（`//TODO: implement consistent read`）。README 宣称的 Strong/Weak/NoGuarantee 三档策略形同虚设。
4. **心跳字段硬编码**：`mod.rs` 心跳里 `"Group": "-1"`、`"NodemapVersion": 1` 都是写死的，从未反映真实分组/版本。
5. **依赖 nightly + 特定 openraft git 版本**：`main.rs` 使用 `#![feature(once_cell)]`、`#![feature(iter_intersperse)]`、`std::lazy::SyncLazy`，**无法在 stable Rust 编译**；`openraft` 通过 git rev `095c1ce...` 锁定。可持续性/可复现性差。
6. **大量 `unwrap()/panic!`**：如 `build_snapshot` 在 `last_applied_log` 为 None 时 `panic!("can not compact empty state machine")`；`send_rpc` 对 `target_node` 直接 `unwrap()`。错误处理薄弱，集群异常场景易崩溃。
7. **命名残留**：sled tree 用 `trylog`/`trymeta`，`open_create` 忽略 `open/create` 参数，从 demo 样例出发痕迹明显。

### 3.4 StorageConnector（Go）
已实现：
- `BasicStorageConnection`：HTTP 连接的构造函数，支持 `WithTimeout`、一致性策略参数。
- `GetSlice` / `PutSlice`；构建 `/slice/<hex>.<encoded_name>?consistency_policy=N` 的 URL。
- 较完整的单元/集成测试（`connector_test.go`：参数校验、超时、响应处理）。

代码问题：
- **地址格式不一致**：`buildAPISliceURL` 无条件拼接 `http://` + `addr`。生产环境 Nodemap 里 addr 是 `127.0.0.1:21001`（无 scheme），可正常工作；但单元测试里传的是 `http://127.0.0.1:port`（`httptest.NewServer` 返回带 scheme 的 URL），会导致 `http://http://...` 双前缀。scheme 处理是隐式约定，未做归一化。
- `validateBasicStorageConnection` 本身标注 `//TODO: not completed`。
- 一致性策略仅作为 URL 参数透传，存储层未消费（见上）。

### 3.5 TLAspec
`TLAspec.tla` 只有 `---- MODULE TLAspec ----` + `====`，**形式化规范为空占位**，未写任何 spec。

### 3.6 e2e_tests
`index.js` 用 Node 写了三个独立的 TCP mock（gateway/index/storage，行协议 `GET/POST`），但**它完全没连真实网关/监控/存储**，只是一套概念演示；甚至 `status[0].push(JSON.parse(...))` 往字符串数组里塞对象、结尾 `http.create` 是不完整代码、transport 层只是玩具。README 声称的「E2E 测试」真正的执行路径其实是 `run.sh` 里的真实组件，`e2e_tests/` 与之无关。

### 3.7 run.sh
- 启动 etcd、gateway、monitor、3 个 storage 节点；提供 `rpc()` 做初始化。
- **硬编码作者机器路径**：`stopall` 里 `killall /home/emon100/source/HADSS/Storage/target/debug/hadss_storage_node`，换机器必挂；且 `killall` 传全路径本就不匹配进程名。
- **关键步骤被注释**：`runall` 中 `# node_group_init` 被注释掉，`try`（节点 4/5/6）也从未调用。也就是说默认 `./run.sh` 只拉起进程、**不主动初始化 Raft 集群**，靠 Monitor 的计算循环尝试凑组，行为不可控。
- 注释掉的 `curl` 演示指令说明作者曾想用 `curl --upload-file` 走 gateway，但未启用。

---

## 4. 愿景 vs 现实：差距与关键缺陷

### 4.1 已经落地的部分（亮点）
- 三层架构骨架 + REST 接口齐备，结构清晰，Golang / Rust / etcd / openraft 选型也符合「分布式存储」方向。
- Gateway 具备一致性哈希定位与缓存 Nodemap、按组重试。
- Monitor 用 etcd 分布式锁实现「多 Monitor 只有一个算」的选主模型，方向正确。
- Storage 用 openraft 实现了**真正的 Raft 复制 + 本地文件落盘**，不是纯 mock。
- StorageConnector 有较好测试覆盖，是唯一有测试的 Go 组件。

### 4.2 与愿景差距最大的点
| 愿景（README） | 实际状态 | 严重度 |
|---|---|---|
| 无单点故障 | 演示仅单实例 etcd + 单 Monitor；Mon 集群虽用锁，但 etcd 本身在脚本里是单点 | 高 |
| Index Layer 宕机仍可应答 | 存储读取直连本地 FS，理论上可行；但 Gateway 依赖 Monitor 的 Nodemap 才能定位节点，Nodemap 不存在则无法路由 | 中 |
| Quorum 读/写 | 写入走 Raft 复制（接近 quorum）；读取**非 quorum**，且忽略一致性策略 | 高 |
| 扩容/缩容数据再平衡 | 只实现「组建新组」，无数据迁移、无缩容；范围切分算法有 bug | 高 |
| 节点故障检测/恢复 | 心跳无 lease，死节点不清理；无 failover | 高 |
| 数据损坏检测/恢复 | 完全未实现 | 高 |
| 可靠快照/日志压缩 | **快照不含文件内容**，节点恢复可能丢数据 | **最高（正确性）** |
| 大文件/流式 | 全量读内存、默认 64KB payload | 中 |
| 形式化验证 | TLA+ 为空 | 中 |
| E2E | mock 与真实系统脱节 | 中 |
| 可复现部署 | nightly Rust + git rev lock + 硬编码路径 | 中 |
| 认证/授权/租户 | 无 | 中（对象存储必备） |
| 可观测性 | 仅 log；无 metrics（除 raft 自带） | 中 |

---

## 5. 未来改进方向（按优先级）

### P0 —— 正确性/数据安全
1. **把文件内容纳入 Raft 复制与快照**：文件字节必须作为 state machine 的一部分（或在快照时一并打包），否则通过快照追平的节点数据不完整。这是当前最需要修复的正确性问题。
2. **失败写入不得进 state machine**：`store_slice` 失败要回滚 `data.push` 并向上返回错误，保证「state machine 记录 = 真实落盘」。
3. **实现一致性读**：读取也走 Raft/版本校验，真正消费 `consistency_policy`（Strong=读 leader/最新 + 多数派；Weak/NoGuarantee 可放宽）。

### P1 —— 可用性/高可用
4. **心跳加 lease/TTL**：让 etcd 能清理过期节点，为「故障检测」提供基础。
5. **实现缩容与数据再平衡**：在两个相邻段之间做「源段收缩、目标段扩展 + 数据迁移 + 回放写」的 reallocate（README 已有算法说明，需落地），并修复范围切分的 double-count / 无法填满哈希环的问题。
6. **把 Monitor 对节点的控制改成异步、幂等、可重试**，避免计算循环里阻塞式 HTTP 顺序调用带来的脆弱性；将「节点分组」决策与「节点自身状态机」解耦。

### P2 —— 工程化/可持续性
7. **去掉 nightly 依赖**：改用稳定 Rust（替换 `once_cell`、`iter_intersperse`、`SyncLazy`），锁一个可构建的 openraft 版本并提交 Cargo.lock（当前 `.gitignore` 忽略了 `Cargo.lock`，不利于可复现构建）。
8. **消灭 `unwrap()/panic`**，用真正的错误处理；`run.sh` 去硬编码、完善 `node_group_init`，补充优雅的启动/停止。
9. **补齐 Gateway / Monitor 单测**，并让 `e2e_tests` 真正驱动真实组件，或删除以免误解。

### P3 —— 面向"对象存储"的产品化
10. **支持流式/大对象**：分片、断点续传、`HEAD`/`DELETE` 语义（README 声称有 HEAD/DELETE 但 slice.rs 只有 GET/PUT）。
11. **一致哈希的虚拟节点（vnodes）**：当前按区间均分，节点增减会大范围重映射；引入虚拟节点可降低迁移开销。
12. **身份认证 / 访问控制 / 租户隔离**、对象元数据（Content-Type、大小、版本）、`Content-Length`、多版本与垃圾回收。
13. **可观测性**：metrics、trace、结构化日志，健康检查 `/health` 全链路。
14. **补齐 TLA+ spec**：对「扩缩容迁移」「组员变更」等关键不变量做形式化验证，能显著降低分布式正确性风险。

---

## 6. 一句话结论

> HADSS 是一个**有清晰愿景、架构方向正确**的分布式存储原型：三层分离、Raft 复制、etcd 协调、一致性哈希路由、锁选主等选型都对。**但当前处于早期草稿阶段**——真正落地且可靠的核心闭环（Quorum 读、数据再平衡、节点故障恢复、包含文件内容的快照）尚未打通，存在「快照不含文件、读取非 quorum、心跳无 TTL、范围切分 bug、依赖 nightly 且错误处理薄弱」等关键缺口。它更像是一份**架构试验田 + 可运行的骨架**，距离「高可用对象存储后端」的愿景还有相当一段工程化与正确性打磨的路要走。
