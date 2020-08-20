FROM ubuntu:20.04
RUN apt-get update && apt-get install -y --no-install-recommends \
	gcc libc6-dev ca-certificates \
	libsystemd-dev pkg-config
ENV PATH=$PATH:/rust/bin
