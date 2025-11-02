# Build
## Dependency Install
```bash
sudo apt install sqlite3 rustup build-essential cmake
rustup default stable
cargo install sqlx-cli --no-default-features --features rustls
```

## Docker Image Build
https://docs.docker.com/engine/install/ubuntu/
```bash
sudo docker build -t nekonic-judge-cpp:latest docker/cpp
sudo docker build -t nekonic-judge-python:latest docker/python
sudo docker build -t nekonic-judge-java:latest docker/java
```
