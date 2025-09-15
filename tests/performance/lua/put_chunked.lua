-- PUT chunked upload test
counter = 0
sizes = {10240, 51200, 102400} -- 10KB, 50KB, 100KB

request = function()
    counter = counter + 1
    local size = sizes[(counter % #sizes) + 1]
    local data = string.rep("C", size)
    local timestamp = os.time()
    local path = "/chunked_file_" .. counter .. "_" .. timestamp .. ".dat"
    
    return wrk.format("PUT", path, {
        ["Content-Type"] = "application/octet-stream"
    }, data)
end