
cargo build --bin dpu-runtime-manager
RUST_LOG=info cargo run --bin dpu-runtime-manager -- --measurement e0f3a02f9f3c85caf8a4753723d7e50ee0f3a02f9f3c85caf8a4753723d7e50e --port 6666 &
RUST_LOG=info cargo run --bin dpu-runtime-manager -- --measurement e0f3a02f9f3c85caf8a4753723d7e50ee0f3a02f9f3c85caf8a4753723d7e50f --port 6667 &

mkdir -p /tmp/dpu;
cd /tmp/dpu;  
openssl ecparam -name prime256v1 -noout -genkey > CAKey.pem
openssl req -x509 -key CAKey.pem -out CACert.pem -config /work/docker/ca-cert.conf

VTS_PATH="/opt/veraison/vts"
PROVISIONING_PATH="/opt/veraison/provisioning"
PAS_PATH="/opt/veraison/proxy_attestation_server"

cd /opt/veraison/vts && /opt/veraison/vts/vts &
cd /opt/veraison/provisioning && /opt/veraison/provisioning/provisioning  &
cd /tmp/dpu &&   /opt/veraison/proxy_attestation_server -l 127.0.0.1:3010 &

cd -
sleep 5

curl -X POST -H 'Content-Type: application/corim-unsigned+cbor; profile=http://arm.com/psa/iot/1' --data-binary "@/opt/veraison/psa_corim.cbor" localhost:8888/endorsement-provisioning/v1/submit

curl -X POST -H 'Content-Type: application/corim-unsigned+cbor; profile=http://aws.com/nitro' --data-binary "@/opt/veraison/nitro_corim.cbor" localhost:8888/endorsement-provisioning/v1/submit

