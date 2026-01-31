cmake_minimum_required(VERSION 3.12)
project(draco)

# Option to choose between building with original src/draco (external) or the refactored draco_core
option(USE_SRC_DRACO "Use original Google Draco sources (src/draco) - completely separate from refactored draco_core" OFF)

if(USE_SRC_DRACO)
  message(STATUS "Building ORIGINAL Google Draco from src/draco (includes its own I/O, tools, etc.)")
  message(STATUS "draco_core and draco_io will NOT be built - they are part of the refactored architecture")
  # Build only the original Google Draco - it has everything built-in
  add_subdirectory(src/draco)
else()
  # Build the refactored architecture: draco_core + draco_io
  message(STATUS "Building REFACTORED Draco architecture: draco_core + draco_io")
  add_subdirectory(draco_core)
  add_subdirectory(draco_io)
endif()
