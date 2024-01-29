# DPU

## Build
```
cargo build
```

## Run
* Run two DPU Runtime Manager instances on the DPU side:
  ```
  RUST_LOG=info cargo run --bin dpu-runtime-manager -- --measurement <hash1> --port <port1>
  RUST_LOG=info cargo run --bin dpu-runtime-manager -- --measurement <hash2> --port <port2>
  ```
* Run the attestation example on the host side:
  ```
  RUST_LOG=info cargo run --bin attestation
  ```

## Run Parsec Example

```
cd docker 
make build
make exec
```

in docker run the script.sh 

```
cd /work;

./script.sh

RUST_LOG=info cargo run --bin parsec_execute
```
