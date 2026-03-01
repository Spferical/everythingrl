#!/bin/bash
sed -i 's/mirrorlist/#mirrorlist/g' /etc/yum.repos.d/CentOS-*
sed -i 's|#baseurl=http://mirror.centos.org|baseurl=http://vault.centos.org|g' /etc/yum.repos.d/CentOS-*

yum install -y \
    openssl-devel \
    alsa-lib-devel \
    libX11-devel \
    libXcursor-devel \
    libXi-devel \
    libXrandr-devel \
    libxcb-devel \
    mesa-libGL-devel \
    fontconfig-devel \
    wayland-devel
