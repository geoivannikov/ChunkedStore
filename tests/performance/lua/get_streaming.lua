-- GET streaming test (incomplete files)
counter = 0
max_files = 50

request = function()
    counter = counter + 1
    local file_id = (counter % max_files) + 1
    local path = "/chunked_file_" .. file_id .. ".dat"
    
    return wrk.format("GET", path)
end