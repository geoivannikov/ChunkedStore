-- DELETE operations test
counter = 0
max_files = 200

request = function()
    counter = counter + 1
    local file_id = (counter % max_files) + 1
    local timestamp = os.time()
    local path = "/file_" .. file_id .. "_" .. timestamp .. ".dat"
    
    return wrk.format("DELETE", path)
end