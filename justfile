check:
	cd backend && cargo fmt --all -- --check
	cd backend && cargo clippy --all-targets --all-features -- -D warnings
	cd backend && cargo test --all --locked
	cd frontend && npm run format:check
	cd frontend && npm run lint
