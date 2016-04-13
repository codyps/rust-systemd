TARGET=${TARGET_ARCH}${TARGET_VENDOR:-}-${TARGET_OS}
host_arch=x86_64
host_os="$TRAVIS_OS_NAME"
case "$host_os" in
  linux)
    host_vendor=-unknown
    host_os=linux-gnu
    ;;
  osx)
    host_vendor=-apple
    host_os=darwin
    ;;
esac
host=${host_arch}${host_vendor:-}-${host_os}
