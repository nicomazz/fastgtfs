# How to build this image:
#
# sudo docker build --tag carlazz/fast-gtfs-release .
# sudo docker push carlazz/fast-gtfs-release

FROM rust:1.57


RUN apt-get update && apt-get install -y --no-install-recommends zip && apt-get clean; \
    curl -s https://api.github.com/repos/tcnksm/ghr/releases/latest | \
    grep browser_download_url | grep linux_amd | cut -d '"' -f 4 | wget -qi -; \
    tar xvzf ghr_v*_linux_amd64.tar.gz; \
    chmod +x ghr_v*_linux_amd64/ghr; \
    cp ghr_v*_linux_amd64/ghr ghr; \
    rm -rf ~/.cache /var/lib/apt/lists/*;


LABEL name="fast-gtfs-release"

CMD ["/bin/bash"]
