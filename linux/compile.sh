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
make && make install
cd ..

# Install static PCRE
wget -O pcre-8.40.tar.gz https://downloads.sourceforge.net/project/pcre/pcre/8.40/pcre-8.40.tar.gz
tar -zxf pcre-8.40.tar.gz
cd pcre-8.40
env CFLAGS="$STATICLIB_CFLAGS" CXXFLAGS="$STATICLIB_CXXFLAGS" \
  ./configure --prefix=/hbb_shlib --disable-shared --enable-static
make
make install
cd ..

#download Epoch server
git clone https://github.com/EpochModTeam/EpochServer.git --recursive
cd EpochServer/
git submodule update --init --recursive
make install

libcheck src/epochserver.so

# Copy result to host
cp src/epochserver.so /io/
