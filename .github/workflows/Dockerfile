# How to build this image:
#
# sudo docker build --tag carlazz/fast-gtfs-release .
# sudo docker push carlazz/fast-gtfs-release

FROM golang:buster

RUN apt-get update && apt-get install -y --no-install-recommends cargo \
    && apt-get clean \
    && go get github.com/tcnksm/ghr \
    && rm -rf ~/.cache /var/lib/apt/lists/*

LABEL name="fast-gtfs-release-ubuntu-21.04"

CMD ["/bin/bash"]