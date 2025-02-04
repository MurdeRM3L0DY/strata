---@class strata
---@field window strata.window

---@class strata.window
local window = {}

---@class strata.window.Handle

--- POC
---@overload fun(event: "WindowEnter", cb: fun(win: strata.window.Handle))
---@overload fun(event: "WindowLeave", cb: fun(win: strata.window.Handle, arg: number))
function window.on(...)
end

-- local api = require("strata.api")
--
-- local module = {}
--
-- --- Moves a window to a different workspace
-- ---@param id number
-- ---@param opts table
-- ---@return function
-- function module.move(id, opts)
--     if opts and opts.follow then
-- 	    return function() api.move_window_and_follow(id) end
--     else
-- 	    return function() api.move_window(id) end
--     end
-- end
--
-- --- Closes the currently active window
-- ---@return function
-- function module.close()
-- 	return function() api.close_window() end
-- end
--
-- return module
