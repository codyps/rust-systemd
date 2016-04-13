#! /bin/sh

set -euf

D=$(dirname "$0")
"$D/systemd/autogen.sh"

mkdir systemd-build

# lto like breaking things on travis (which is the primary use of this build script), so we'll
# disable it using the config-cache
#
# Based on http://www.linuxfromscratch.org/lfs/view/systemd/chapter06/systemd.html
cat >>systemd-build/config.cache <<EOF
cc_cv_CFLAGS__flto=no
EOF

src="$(realpath --relative-to=./systemd-build "$D"/systemd)"

cd systemd-build
"$src/configure" \
	--config-cache \
	--enable-kdbus \
	--disable-tests \
	--disable-ldconfig \
	--without-python \
	--disable-sysusers \
	--disable-firstboot \
	--disable-manpages

j="$(getconf _NPROCESSORS_ONLN)"

make -j$j

sudo make install
