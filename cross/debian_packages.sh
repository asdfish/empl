#!/bin/sh

dpkg --add-architecutre ${CROSS_DEB_ARCH}
apt-get update
apt-get install libasound2-dev:${CROSS_DEB_ARCH} --yes
