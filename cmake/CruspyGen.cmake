# EP-0021: run cruspy-gen before IDE/clangd builds.
function(cruspy_gen_models CRUSPY_ROOT)
    set(_GEN_SCRIPT "${CMAKE_CURRENT_SOURCE_DIR}/tools/cruspy-gen/cruspy_gen.py")
    find_program(UV_EXECUTABLE uv)
    if(UV_EXECUTABLE)
        set(_GEN_CMD
            "${UV_EXECUTABLE}" run --with pyyaml --with jinja2 python "${_GEN_SCRIPT}"
            --root "${CRUSPY_ROOT}"
            --glob "models/**/*.openapi.yaml"
        )
    else()
        set(_GEN_CMD
            python3 "${_GEN_SCRIPT}"
            --root "${CRUSPY_ROOT}"
            --glob "models/**/*.openapi.yaml"
        )
    endif()
    add_custom_target(cruspy_gen ALL
        COMMAND ${_GEN_CMD}
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
        COMMENT "Running cruspy-gen (OpenAPI → C++/Rust/Python)"
        VERBATIM
    )
endfunction()
