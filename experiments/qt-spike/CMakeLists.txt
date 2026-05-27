cmake_minimum_required(VERSION 3.20)
project(BetterCPT_QtSpike LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets)

add_executable(qt-spike
    main.cpp
    RenderServer.cpp
    RenderServer.h
    WidgetManager.cpp
    WidgetManager.h
)

target_link_libraries(qt-spike Qt6::Widgets)
