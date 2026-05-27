cmake_minimum_required(VERSION 3.20)
project(BetterCPT_WidgetQt LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets)

add_executable(widget-qt WIN32
    main.cpp
    RenderServer.h
    RenderServer.cpp
    WidgetManager.h
    WidgetManager.cpp
    RoundedWidget.h
    RoundedWidget.cpp
)

target_link_libraries(widget-qt Qt6::Widgets)
