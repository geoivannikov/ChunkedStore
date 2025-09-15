-- small_chunked_test.lua
counter = 0

request = function()
    counter = counter + 1
    local sizes = {100, 1024} -- Только 100B и 1KB
    local size = sizes[(counter % #sizes) + 1]
    local data = string.rep("A", size)
    local path = "/small_file_" .. counter .. ".dat"
    
    return wrk.format("PUT", path, {
        ["Content-Type"] = "application/octet-stream"
    }, data)
end