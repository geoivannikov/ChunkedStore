-- Video streaming test
counter = 0
max_segments = 50

request = function()
    counter = counter + 1
    local segment_id = (counter % max_segments) + 1
    local path = "/video/segment_" .. string.format("%03d", segment_id) .. ".m4s"
    
    return wrk.format("GET", path)
end