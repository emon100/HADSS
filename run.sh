#! /bin/bash
readonly GATEWAY_LISTEN_ADDR="0.0.0.0:9999"
readonly MONITOR_ADDR="http://localhost:10000"
readonly MONITOR_LISTEN_ADDR="0.0.0.0:10000"

trap stopall INT EXIT

export RUST_LOG=debug
export RUST_BACKTRACE=1

rpc() {
    local uri=$1
    local body="$2"

    echo '---'" rpc(:$uri, $body)"

    {
        if [ ".$body" = "." ]; then
            curl --silent "127.0.0.1:$uri"
        else
            curl --silent "127.0.0.1:$uri" -H "Content-Type: application/json" -d "$body"
        fi
    } | {
        if type jq > /dev/null 2>&1; then
            jq
        else
            cat
        fi
    }

    echo
    echo
}

gateway() {
  pushd Gateway &&
	go build -o /tmp/gateway &&
	popd &&
	/tmp/gateway -listenAddr $GATEWAY_LISTEN_ADDR -monitorAddr $MONITOR_ADDR &
}

monitor() {
  pushd Monitor &&
  go build -o /tmp/monitor &&
  popd &&
  /tmp/monitor -listenAddr $MONITOR_LISTEN_ADDR &
}

storage() {
  pushd Storage
  cargo build
	cargo run -- --port 21001 --node-id 1 --node-addr "127.0.0.1:21001" --monitor-addr "localhost:10000" --storage-location "/tmp/storage/node1" &
	cargo run -- --port 21002 --node-id 2 --node-addr "127.0.0.1:21002" --monitor-addr "localhost:10000" --storage-location "/tmp/storage/node2" &
	cargo run -- --port 21003 --node-id 3 --node-addr "127.0.0.1:21003" --monitor-addr "localhost:10000" --storage-location "/tmp/storage/node3" &
	sleep 1
	popd
}

stopall() {
  echo "Stopping..."
  killall gateway
  killall monitor
  killall /home/emon100/source/HADSS/Storage/target/debug/hadss_storage_node
  echo "Stopped all"
  exit 0
}

node_group_init() {
  rpc 21001/init '{}'

  echo "Server 1 is a leader now"

  echo "Get metrics from the leader"
  sleep 2
  echo
  rpc 21001/metrics

  echo "Adding node 2 and node 3 as learners, to receive log from leader node 1"

  rpc 21001/add-learner       '[2, "127.0.0.1:21002"]'
  echo "Node 2 added as leaner"
  sleep 1
  echo
  rpc 21001/add-learner       '[3, "127.0.0.1:21003"]'
  echo "Node 3 added as leaner"

  echo "Get metrics from the leader, after adding 2 learners"
  echo
  rpc 21001/metrics
  sleep 1

  echo "Changing membership from [1] to 3 nodes cluster: [1, 2, 3]"
  echo
  rpc 21001/change-membership '[1, 2, 3]'
  echo "Membership changed"

  echo "Get metrics from the leader again"
  echo
  rpc 21001/metrics
  sleep 1
}

runall() {
  etcd –data-dir ~/default.etcd
  gateway
  monitor
  storage
# node_group_init

# curl http://localhost:9999/id/trycpp --upload-file ~/try.cpp
# curl http://localhost:9999/id/trycpp


  sleep infinity
}

runall
