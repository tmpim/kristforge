cmake_minimum_required(VERSION 3.10)
project(kristforge)

set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_SOURCE_DIR}/cmake/Modules")

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

## Process Embeddeds ##

set(EMBEDDED_INPUT kristforge.cl)
find_program(XXD_EXE xxd
	DOC "Location of the xxd executable"
)

macro(ProcessWithXXD outvar inputfile outdir)
	get_filename_component(basefile ${inputfile} NAME)
	set(outputfile ${outdir}/${basefile}.xxd)
	set_source_files_properties(${outputfile} PROPERTIES HEADER_ONLY TRUE)
	add_custom_command(
		OUTPUT ${outputfile}
		COMMAND ${XXD_EXE} -i ${inputfile} ${outputfile}
		WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
		DEPENDS ${inputfile}
		COMMENT "Generating XXD for ${inputfile}"
	)
	list(APPEND ${outvar} ${outputfile})
endmacro()

foreach(item ${EMBEDDED_INPUT})
	ProcessWithXXD(EMBEDDED_SOURCE ${item} ${CMAKE_CURRENT_BINARY_DIR})
endforeach()

set(CMAKE_INCLUDE_CURRENT_DIR ON)
source_group("Embedded Source" FILES ${EMBEDDED_INPUT} ${EMBEDDED_SOURCE})

add_executable(kristforge main.cpp state.cpp state.h network.cpp network.h ${EMBEDDED_SOURCE} miner.cpp miner.h cl_amd.h cl_nv.h utils.cpp utils.h)

find_package(OpenCL REQUIRED)
target_include_directories(kristforge PUBLIC ${OpenCL_INCLUDE_DIR})
target_link_libraries(kristforge PUBLIC ${OpenCL_LIBRARIES})

set(THREADS_PREFER_PTHREAD_FLAG ON)
find_package(Threads REQUIRED)
target_link_libraries(kristforge PRIVATE Threads::Threads)

find_package(TCLAP REQUIRED)
target_include_directories(kristforge PRIVATE ${TCLAP_INCLUDE_DIR})

find_package(OpenSSL REQUIRED)
target_include_directories(kristforge PRIVATE ${OPENSSL_INCLUDE_DIR})
target_link_libraries(kristforge PRIVATE ${OPENSSL_SSL_LIBRARY})
target_link_libraries(kristforge PRIVATE ${OPENSSL_CRYPTO_LIBRARY})

find_package(CURL REQUIRED)
target_include_directories(kristforge PRIVATE ${CURL_INCLUDE_DIR})
target_link_libraries(kristforge PRIVATE ${CURL_LIBRARY})

pkg_check_modules(JSONCPP jsoncpp)
target_include_directories(kristforge PRIVATE ${JSONCPP_INCLUDEDIR})
target_link_libraries(kristforge PRIVATE ${JSONCPP_LIBRARIES})

find_path(UWEBSOCKETS_INCLUDE_DIRS uWS)
find_library(UWEBSOCKETS_LIBRARIES uWS)
target_include_directories(kristforge PRIVATE ${UWEBSOCKETS_INCLUDE_DIRS})
target_link_libraries(kristforge PRIVATE ${UWEBSOCKETS_LIBRARIES})

# dependency of uwebsockets
find_package(ZLIB REQUIRED)
target_link_libraries(kristforge PRIVATE ${ZLIB_LIBRARIES})
