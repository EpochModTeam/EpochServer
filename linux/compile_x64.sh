#!/bin/bash
set -ex

# instructions here: https://github.com/EpochModTeam/EpochServer/wiki/EpochServer-Build-Notes

# Activate Holy Build Box environment.
source /hbb_shlib/activate

# install git
yum -y install git wget

# install hiredis
#unzip hiredis-master.zip
git clone https://github.com/redis/hiredis.git
cd hiredis
make
make install PREFIX=/hbb_shlib
cd ..

# Install static PCRE
PCRE_VER=8.40
wget -O pcre-$PCRE_VER.tar.gz https://downloads.sourceforge.net/project/pcre/pcre/$PCRE_VER/pcre-$PCRE_VER.tar.gz
tar -zxf pcre-$PCRE_VER.tar.gz
cd pcre-$PCRE_VER
env CFLAGS="$STATICLIB_CFLAGS" CXXFLAGS="$STATICLIB_CXXFLAGS" \
  ./configure --prefix=/hbb_shlib --disable-shared --enable-static
make
make install
cd ..

#download Epoch server
git clone https://github.com/EpochModTeam/EpochServer.git --recursive
cd EpochServer/
git submodule update --init --recursive

# overide makefile
cp /io/Makefile src/

# build epochserver lib
make install

libcheck src/epochserver.so
ldd src/epochserver.so
arch

# Copy result to host
cp src/epochserver.so /io/epochserver_x64.so
