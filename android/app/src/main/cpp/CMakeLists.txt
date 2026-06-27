cmake_minimum_required(VERSION 3.18.1)

project(LXMusic)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(
    lxmusic-turbo
    SHARED
    jni/lx_music_jni.cpp
)

find_library(
    log-lib
    log
)

target_link_libraries(
    lxmusic-turbo
    ${log-lib}
)