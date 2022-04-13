#! /bin/bash
readonly GATEWAY_LISTEN_ADDR="0.0.0.0:9999"
readonly MONITOR_ADDR="http://localhost:8500/v1/kv"
GATEWAY_PID=" "
STORAGE_PID=" "

trap stopall INT EXIT

gateway() {
  pushd Gateway
	go build -o /tmp/gateway
	popd
	/tmp/gateway -listenAddr $GATEWAY_LISTEN_ADDR -monitorAddr $MONITOR_ADDR &
	GATEWAY_PID=$!
}

storage() {
  pushd Storage
	cargo run -- --port 12321 &
	STORAGE_PID=$!
	popd
}

stopall() {
  echo
  echo "Stopping $GATEWAY_PID $STORAGE_PID ..."
  kill $GATEWAY_PID $STORAGE_PID
  echo "Stopped all"
  exit 0
}

runall(){
  gateway
  storage
  sleep infinity
}

runall
