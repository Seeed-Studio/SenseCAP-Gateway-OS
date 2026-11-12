#!/usr/bin/env lua

require "ubus"
require "uloop"
local json = require "luci.jsonc"
local fs   = require "nixio.fs"

uloop.init()

local conn = ubus.connect()
if not conn then
        error("Failed to connect to ubus")
end

local gps_longitude_v = 0
local gps_latitude_v = 0
local gps_altitude_v = 0
local gps_time_v = 0
local gps_state_v = 0

local lora_tx_sum_v = 0
local lora_rx_sum_v = 0
local lora_report_time_v = 0
local lora_temperature_v = 0
local lora_state_v = 0

local net_state_v = 0

local lora_module_state = 1
local  lora_network_connect = {
        lora_pkt_fwd = 0,
        station = 0
}

local lora_history = {}
local lora_history_last_save_time = 0

-- load lora history data from disk
local function try_load_lora_history_from_disk(file_path)
        local fd_his, err = io.open(file_path, "r")
        if fd_his then
                local content = fd_his:read("*a")
                print("content of "..file_path..": "..content)
                local lora_his = json.parse(content)
                if lora_his then
                        if lora_his.data and lora_his.nearest_ts and #lora_his.data == 24 then
                                lora_history = lora_his
                                print("loaded lora history from disk")
                                lora_history_last_save_time = os.time()
                        end
                end
                fd_his:close()
        end
end

try_load_lora_history_from_disk("/usr/share/lora-history.json")
if not lora_history.data then
        -- maybe rename was interrupted, restore backup file
        try_load_lora_history_from_disk("/usr/share/lora-history.json.bak")
end

-- still no history? very initial state / json file corrupted somehow rarely
if not lora_history.data then
        -- init empty lora history
        local now = os.time()
        local ts_hour = now - (now % 3600) -- into hour edge
        lora_history.nearest_ts = ts_hour
        lora_history.data = {}
        for i=1,24 do
                lora_history.data[i] = {0,0}
        end
end

function save_lora_history_to_disk()
        -- fill json.tmp
        local content = json.stringify(lora_history)
        print("save lora_history to disk:"..content)
        fs.writefile("/usr/share/lora-history.json.tmp", content)
        -- mv json json.bak
        fs.move("/usr/share/lora-history.json", "/usr/share/lora-history.json.bak")
        -- mv json.tmp json
        fs.move("/usr/share/lora-history.json.tmp", "/usr/share/lora-history.json")
        -- rm json.bak
        fs.unlink("/usr/share/lora-history.json.bak")
end

function maybe_shift_lora_history (timestamp, num_rx, num_tx)
        local now = os.time()
        local ts_hour = now - (now % 3600) -- into hour edge

        if ts_hour > lora_history.nearest_ts then
                -- new hour, need re-order
                local t = ts_hour
                local i,j,k
                local data = {}
                for i=1,24 do
                        if t <= lora_history.nearest_ts then
                                local k = 1
                                for j=i,24 do
                                        data[j] = lora_history.data[k]
                                        k = k + 1
                                end
                                break
                        else
                                data[i] = {0,0}
                        end
                        t = t - 3600
                end
                lora_history.data = data
                lora_history.nearest_ts = ts_hour
        end

        if timestamp then
                local t = lora_history.nearest_ts
                for i=1,24 do
                        if timestamp > t then
                                local rx = lora_history.data[i][1]
                                local tx = lora_history.data[i][2]
                                rx = rx + num_rx
                                tx = tx + num_tx
                                lora_history.data[i] = {rx, tx}
                                break
                        end
                        t = t - 3600
                end
        end

        -- check if we should save it to disk
        if now - lora_history_last_save_time > 3600 then
                save_lora_history_to_disk()
                lora_history_last_save_time = now
                print("saved lora history to disk")
        end
end

-- tidy the history data on start up
maybe_shift_lora_history()


-- register exit cleanup
local function atexit_cleanup()
    save_lora_history_to_disk()
end

-- atexit handlers are run when this object gets GCed
gcobj = newproxy(true)
getmetatable(gcobj).__gc = atexit_cleanup

-- Hook os.exit
-- Look away now. This is pretty gross.
-- Inspired by Tcl!
local real_os_exit = os.exit
function os.exit(...)
    atexit_cleanup()
    return real_os_exit(...)
end


local my_method = {
        sensecap = {
                gps = {
                        function(req, msg)
                                print("Call to function 'gps'")
                                for k, v in pairs(msg) do
                                        print("key=" .. k .. " value=" .. v)
                                        if( k == "longitude" )
                                        then
                                                gps_longitude_v = v
                                        end
                                        if( k == "latitude" )
                                        then
                                                gps_latitude_v = v
                                        end
                                        if( k == "altitude" )
                                        then
                                                gps_altitude_v = v
                                        end
                                        if( k == "state" )
                                        then
                                                gps_state_v = v
                                        end
                                        if( k == "gps_time" )
                                        then
                                                gps_time_v = v
                                        end
                                end
                                conn:reply(req, {longitude=gps_longitude_v,latitude=gps_latitude_v,altitude=gps_altitude_v,state=gps_state_v,gps_time=gps_time_v});
                        end, {longitude=ubus.STRING,latitude=ubus.STRING,altitude=ubus.STRING,state=ubus.INT8,gps_time=INT32}
                },
                lora = {
                        function(req, msg)
                                print("Call to function 'lora_statistics'")
                                for k, v in pairs(msg) do
                                        print("key=" .. k .. " value=" .. v)
                                        if( k == "rx_sum" )
                                        then
                                                lora_rx_sum_v = v
                                        end
                                        if( k == "tx_sum" )
                                        then
                                                lora_tx_sum_v = v
                                        end
                                        if( k == "temperature" )
                                        then
                                                lora_temperature_v = v
                                        end
                                        if( k == "report_time" )
                                        then
                                                lora_report_time_v = v
                                        end
                                        if( k == "state" )
                                        then
                                                lora_state_v = v
                                        end
                                end
                                conn:reply(req, {rx_sum=lora_rx_sum_v,tx_sum=lora_tx_sum_v,temperature=lora_temperature_v,report_time=lora_report_time_v,state=lora_state_v});
                                maybe_shift_lora_history(lora_report_time_v, lora_rx_sum_v, lora_tx_sum_v)
                        end, {rx_sum=ubus.INT32,tx_sum=ubus.INT32,temperature=ubus.STRING,report_time=ubus.INT32,state=ubus.INT8}
                },
                lora_history = {
                        function(req, msg)
                                print("Calling function 'lora_history'...")
                                conn:reply(req, lora_history)
                                maybe_shift_lora_history()
                        end, {data=ubus.ARRAY, nearest_ts=ubus.INT32}
                },
                net_state = {
                        function(req, msg)
                                print("Calling function 'net_state'...")
                                for k, v in pairs(msg) do
                                        print("key=" .. k .. " value=" .. v)
                                        if k == "state" then
                                                net_state_v = v
                                        end
                                end
                                conn:reply(req, {state=net_state_v});
                        end, {state=ubus.INT8}
                },
                lora_network_connect = {
                        function(req, msg)
                                print("Call to function 'lora_network_connect'")
                                for k, v in pairs(msg) do
                                        print("key=" .. k .. " value=" .. v)
                                        if( k == "lora_pkt_fwd" )
                                        then
                                                lora_network_connect.lora_pkt_fwd = v
                                        end
                                        if( k == "station" )
                                        then
                                                lora_network_connect.station = v
                                        end
                                end
                                conn:reply(req, lora_network_connect);
                        end, {lora_pkt_fwd=ubus.INT8, station=ubus.INT8}

                },
                lora_module = {
                        function(req, msg)
                                print("Calling function 'lora_module'...")
                                for k, v in pairs(msg) do
                                        print("key=" .. k .. " value=" .. v)
                                        if k == "state" then
                                                lora_module_state = v
                                        end
                                end
                                conn:reply(req, {state=lora_module_state});
                        end, {state=ubus.INT8}
                },
                reboot_host = {
                        function(req, msg)
                                conn:reply(req, {result=0})
                                uloop.timer(function()
                                        print("Rebooting host...")
                                        os.execute("sync && echo s > /proc/sysrq-trigger && echo b > /proc/sysrq-trigger")
                                end, 2000)
                        end, {}
                },
                shutdown_host = {
                        function(req, msg)
                                conn:reply(req, {result=0})
                                uloop.timer(function()
                                        print("Shutting down host...")
                                        os.execute("sync && echo o > /proc/sysrq-trigger")
                                end, 2000)
                        end, {}
                }
        }
}

conn:add(my_method)

uloop.run()