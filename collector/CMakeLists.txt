project(fuzzmon_collector)

add_custom_target(collector_lib
  COMMAND cargo build --release --lib --target-dir ${CMAKE_CURRENT_BINARY_DIR}
  WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)
set_property(TARGET collector_lib PROPERTY
  FUZZMON_COLLECTOR_LIB_PATH ${CMAKE_CURRENT_BINARY_DIR}/release/libcollector.so)
