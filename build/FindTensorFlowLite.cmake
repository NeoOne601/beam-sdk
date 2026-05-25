# FindTensorFlowLite.cmake
# Custom finder for TensorFlow Lite on Android

find_path(TensorFlowLite_INCLUDE_DIR
    NAMES tensorflow/lite/interpreter.h
    PATHS ${CMAKE_CURRENT_SOURCE_DIR}/../tflite/include
    NO_CMAKE_FIND_ROOT_PATH
)

find_library(TensorFlowLite_LIBRARY
    NAMES tensorflowlite tensorflowlite_jni
    PATHS ${CMAKE_CURRENT_SOURCE_DIR}/../tflite/lib/${ANDROID_ABI}
    NO_CMAKE_FIND_ROOT_PATH
)

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(TensorFlowLite
    REQUIRED_VARS TensorFlowLite_LIBRARY TensorFlowLite_INCLUDE_DIR
)

if (TensorFlowLite_FOUND)
    set(TensorFlowLite_LIBRARIES ${TensorFlowLite_LIBRARY})
    set(TensorFlowLite_INCLUDE_DIRS ${TensorFlowLite_INCLUDE_DIR})

    if (NOT TARGET TensorFlowLite::TensorFlowLite)
        add_library(TensorFlowLite::TensorFlowLite SHARED IMPORTED)
        set_target_properties(TensorFlowLite::TensorFlowLite PROPERTIES
            IMPORTED_LOCATION "${TensorFlowLite_LIBRARY}"
            INTERFACE_INCLUDE_DIRECTORIES "${TensorFlowLite_INCLUDE_DIR}"
        )
    endif()
endif()
