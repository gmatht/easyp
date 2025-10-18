REM This is the only script that I think is needed to build redox. I will cleanup the others later.
cd redox
docker pull redoxos/redoxer
REM only tested in interactive mode: docker run -v D:\src\easyp:/easyp -it docker.io/redoxos/redoxer:latest
REM docker run -v D:\src\easyp:/easyp -t docker.io/redoxos/redoxer:latest bash -c 'cd /easyp/redox;redoxer build;../gz99 < ../target/x86_64-unknown-redox/lto/easyp easyp-0.1.3-alpha2.gz'
docker run -v D:\src\easyp:/easyp -t docker.io/redoxos/redoxer:latest bash -c "cd /easyp;redoxer build"
REM cargo build && cp target/x86_64-unknown-redox/debug/easyp /tmp/redox/r && redoxer exec -f /tmp/redox /tmp/redox/redox/r
