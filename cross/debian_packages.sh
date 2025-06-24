#!/bin/sh

dpkg --add-architecture ${CROSS_DEB_ARCH}
apt-get update
apt-get install libasound2-dev:${CROSS_DEB_ARCH} --yes
