# FindDracoRust.cmake - Find Draco Rust components
#
# This module defines:
#  DRACO_RUST_FOUND - whether Draco Rust components were found
#  DRACO_RUST_INCLUDE_DIRS - include directories for Draco Rust headers
#  DRACO_RUST_LIBRARIES - libraries to link against
#  DRACO_RUST_VERSION - version of Draco Rust components
#
# It also defines the following imported targets:
#  DracoRust::draco_core - Draco core Rust library

include(FindPackageHandleStandardArgs)

# Try to find the include directory
find_path(DRACO_RUST_INCLUDE_DIR
    NAMES draco_core.h
    PATHS
        ${CMAKE_CURRENT_SOURCE_DIR}/crates/draco-core/include
        ${CMAKE_CURRENT_SOURCE_DIR}/../crates/draco-core/include
        ${DRACO_RUST_ROOT}/include
        ENV DRACO_RUST_ROOT/include
        /usr/local/include
        /usr/include
    DOC "Draco Rust include directory"
)

# Try to find the static library
find_library(DRACO_RUST_CORE_LIBRARY
    NAMES draco_core libdraco_core draco_core_static
    PATHS
        ${CMAKE_CURRENT_SOURCE_DIR}/crates/draco-core/target/debug
        ${CMAKE_CURRENT_SOURCE_DIR}/crates/draco-core/target/release
        ${CMAKE_CURRENT_SOURCE_DIR}/../crates/draco-core/target/debug
        ${CMAKE_CURRENT_SOURCE_DIR}/../crates/draco-core/target/release
        ${DRACO_RUST_ROOT}/lib
        ENV DRACO_RUST_ROOT/lib
        /usr/local/lib
        /usr/lib
    DOC "Draco Rust core library"
)

# Check if we found the required components
find_package_handle_standard_args(DracoRust
    REQUIRED_VARS DRACO_RUST_INCLUDE_DIR DRACO_RUST_CORE_LIBRARY
    VERSION_VAR DRACO_RUST_VERSION
)

if(DRACO_RUST_FOUND)
    # Set the standard variables
    set(DRACO_RUST_INCLUDE_DIRS ${DRACO_RUST_INCLUDE_DIR})
    set(DRACO_RUST_LIBRARIES ${DRACO_RUST_CORE_LIBRARY})

    # Create imported target for the core library
    if(NOT TARGET DracoRust::draco_core)
        add_library(DracoRust::draco_core STATIC IMPORTED)
        set_target_properties(DracoRust::draco_core PROPERTIES
            IMPORTED_LOCATION "${DRACO_RUST_CORE_LIBRARY}"
            INTERFACE_INCLUDE_DIRECTORIES "${DRACO_RUST_INCLUDE_DIR}"
            INTERFACE_COMPILE_DEFINITIONS "DRACO_RUST_CORE=1"
        )

        # Add link dependencies that Rust libraries might need
        if(WIN32)
            set_property(TARGET DracoRust::draco_core APPEND PROPERTY
                INTERFACE_LINK_LIBRARIES "ws2_32;bcrypt;userenv")
        elseif(APPLE)
            set_property(TARGET DracoRust::draco_core APPEND PROPERTY
                INTERFACE_LINK_LIBRARIES "pthread;dl;m")
        else()
            set_property(TARGET DracoRust::draco_core APPEND PROPERTY
                INTERFACE_LINK_LIBRARIES "pthread;dl")
        endif()
    endif()

    # Create a function to build the Rust library
    function(build_draco_rust_core)
        set(options RELEASE DEBUG STATIC SHARED)
        set(oneValueArgs CRATE_DIR TARGET_NAME)
        set(multiValueArgs CARGO_FLAGS)
        cmake_parse_arguments(BUILD "${options}" "${oneValueArgs}" "${multiValueArgs}" ${ARGN})

        # Default values
        if(NOT BUILD_CRATE_DIR)
            set(BUILD_CRATE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/crates/draco-core)
        endif()

        if(NOT BUILD_TARGET_NAME)
            set(BUILD_TARGET_NAME draco_core_rust)
        endif()

        # Determine build mode
        if(BUILD_RELEASE)
            set(CARGO_PROFILE --release)
            set(TARGET_DIR release)
        else()
            set(CARGO_PROFILE)
            set(TARGET_DIR debug)
        endif()

        # Determine library type
        if(BUILD_SHARED)
            set(LIB_TYPE cdylib)
        else()
            set(LIB_TYPE staticlib)
        endif()

        # Add the build target
        add_custom_command(
            OUTPUT ${CMAKE_BINARY_DIR}/libdraco_core.a
            COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_BINARY_DIR}
                cargo build ${CARGO_PROFILE} --manifest-path ${BUILD_CRATE_DIR}/Cargo.toml
                --library-type=${LIB_TYPE} ${BUILD_CARGO_FLAGS}
            WORKING_DIRECTORY ${BUILD_CRATE_DIR}
            DEPENDS ${BUILD_CRATE_DIR}/Cargo.toml ${BUILD_CRATE_DIR}/src/lib.rs
            COMMENT "Building Draco Rust core library"
        )

        add_custom_target(${BUILD_TARGET_NAME} ALL
            DEPENDS ${CMAKE_BINARY_DIR}/libdraco_core.a
        )

        # Create an imported target for the built library
        add_library(draco_core_built STATIC IMPORTED)
        set_target_properties(draco_core_built PROPERTIES
            IMPORTED_LOCATION ${CMAKE_BINARY_DIR}/libdraco_core.a
            INTERFACE_INCLUDE_DIRECTORIES ${BUILD_CRATE_DIR}/include
            INTERFACE_COMPILE_DEFINITIONS "DRACO_RUST_CORE=1"
        )

        # Add dependencies for platform-specific libraries
        if(WIN32)
            set_property(TARGET draco_core_built APPEND PROPERTY
                INTERFACE_LINK_LIBRARIES "ws2_32;bcrypt;userenv")
        else()
            set_property(TARGET draco_core_built APPEND PROPERTY
                INTERFACE_LINK_LIBRARIES "pthread;dl")
        endif()
    endfunction()

    # Function to add Rust support to a target
    function(target_link_draco_rust TARGET)
        cmake_parse_arguments(LINK "PRIVATE;PUBLIC" "" "" ${ARGN})

        if(DRACO_RUST_FOUND)
            if(LINK_PUBLIC)
                target_link_libraries(${TARGET} PUBLIC DracoRust::draco_core)
                target_include_directories(${TARGET} PUBLIC ${DRACO_RUST_INCLUDE_DIR})
                target_compile_definitions(${TARGET} PUBLIC DRACO_RUST_CORE=1)
            else()
                target_link_libraries(${TARGET} PRIVATE DracoRust::draco_core)
                target_include_directories(${TARGET} PRIVATE ${DRACO_RUST_INCLUDE_DIR})
                target_compile_definitions(${TARGET} PRIVATE DRACO_RUST_CORE=1)
            endif()
        else()
            message(WARNING "Draco Rust components not found. C++ implementations will be used.")
        endif()
    endfunction()

    # Function to create compile-time switches
    function(add_draco_rust_switches TARGET)
        if(DRACO_RUST_FOUND)
            # Add compile definitions for conditional compilation
            target_compile_definitions(${TARGET} PRIVATE DRACO_USE_RUST=1)

            # Create a configuration header for C++
            set(CONFIG_HEADER ${CMAKE_CURRENT_BINARY_DIR}/draco_rust_config.h)
            configure_file(
                ${CMAKE_CURRENT_SOURCE_DIR}/cmake/draco_rust_config.h.in
                ${CONFIG_HEADER}
                @ONLY
            )

            target_include_directories(${TARGET} PRIVATE ${CMAKE_CURRENT_BINARY_DIR})
        endif()
    endfunction()

endif()

# Mark variables as advanced
mark_as_advanced(
    DRACO_RUST_INCLUDE_DIR
    DRACO_RUST_CORE_LIBRARY
    DRACO_RUST_VERSION
)