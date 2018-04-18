cmake_minimum_required(VERSION 3.10)
project(kristforge)

set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_SOURCE_DIR}/cmake/Modules")

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

FUNCTION(ADD_RESOURCES out_var)
	SET(result)
	FOREACH(in_f ${ARGN})
		FILE(RELATIVE_PATH src_f ${CMAKE_SOURCE_DIR} ${CMAKE_CURRENT_SOURCE_DIR}/${in_f})
		SET(out_f "${PROJECT_BINARY_DIR}/${in_f}.o")
		ADD_CUSTOM_COMMAND(OUTPUT ${out_f}
				COMMAND ld -r -b binary -o ${out_f} ${src_f}
				DEPENDS ${in_f}
				WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
				COMMENT "Building GLSL object ${out_f}"
				VERBATIM
				)
		LIST(APPEND result ${out_f})
	ENDFOREACH()
	SET(${out_var} "${result}" PARENT_SCOPE)
ENDFUNCTION()

ADD_RESOURCES(CL_SOURCE kristforge.cl)

add_executable(kristforge main.cpp state.cpp state.h network.cpp network.h ${CL_SOURCE})

find_package(OpenCL REQUIRED)
target_include_directories(kristforge PUBLIC ${OpenCL_INCLUDE_DIR})
target_link_libraries(kristforge PUBLIC ${OpenCL_LIBRARIES})

set(THREADS_PREFER_PTHREAD_FLAG ON)
find_package(Threads REQUIRED)
target_link_libraries(kristforge PRIVATE Threads::Threads)

find_package(TCLAP REQUIRED)
target_include_directories(kristforge PRIVATE ${TCLAP_INCLUDE_DIR})

pkg_check_modules(CURLPP REQUIRED curlpp)
target_link_libraries(kristforge PRIVATE ${CURLPP_LDFLAGS})

pkg_check_modules(JSONCPP jsoncpp)
target_link_libraries(kristforge PRIVATE ${JSONCPP_LIBRARIES})

find_path(UWEBSOCKETS_INCLUDE_DIRS uWS)
find_library(UWEBSOCKETS_LIBRARIES uWS)
target_include_directories(kristforge PRIVATE ${UWEBSOCKETS_INCLUDE_DIRS})
target_link_libraries(kristforge PRIVATE ${UWEBSOCKETS_LIBRARIES})

# dependencies of uwebsockets
find_package(ZLIB REQUIRED)
target_link_libraries(kristforge PRIVATE ${ZLIB_LIBRARIES})

find_package(OpenSSL REQUIRED)
target_link_libraries(kristforge PRIVATE ${OPENSSL_SSL_LIBRARY})
target_link_libraries(kristforge PRIVATE ${OPENSSL_CRYPTO_LIBRARY})