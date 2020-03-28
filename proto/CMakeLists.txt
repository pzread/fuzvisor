set(PROTOBUF_PROTOC "${protobuf_BINARY_DIR}/protoc")

set(PROTOS control_flow_graph.proto)

foreach(PROTO ${PROTOS})
    get_filename_component(PROTO_PATH "${PROTO}" ABSOLUTE)
    get_filename_component(PROTO_NAME "${PROTO}" NAME_WLE)
    list(APPEND PROTO_PATHS "${PROTO_PATH}")
    list(APPEND PROTO_SRCS "${CMAKE_CURRENT_BINARY_DIR}/${PROTO_NAME}.pb.cc")
    list(APPEND PROTO_HDRS "${CMAKE_CURRENT_BINARY_DIR}/${PROTO_NAME}.pb.h")
endforeach()

add_custom_command(
    OUTPUT "${PROTO_SRCS}" "${PROTO_HDRS}"
    COMMAND ${PROTOBUF_PROTOC}
    ARGS --cpp_out "${CMAKE_CURRENT_BINARY_DIR}"
        -I "${CMAKE_CURRENT_SOURCE_DIR}"
        "${PROTO_PATH}"
    DEPENDS "${PROTOS}")

add_custom_target(proto ALL DEPENDS "${PROTO_SRCS}" "${PROTO_HDRS}")

set(PROTO_DIR "${CMAKE_CURRENT_BINARY_DIR}" PARENT_SCOPE)
