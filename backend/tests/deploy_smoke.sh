cargo run --bin create_account -- alice P@ssw0rd "Alice Chen" admin

DEPLOY_RUN_FULL=1 \
DEPLOY_BASE_URL=http://localhost:8080 \
AWS_PUBLIC_ENDPOINT_URL=http://localhost:8080
DEPLOY_ACCOUNT=alice \
DEPLOY_PASSWORD='P@ssw0rd' \
cargo test --test deploy_smoke -- --ignored --nocapture
