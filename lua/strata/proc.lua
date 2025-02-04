---@diagnostic disable: missing-return

---@class strata
---@field proc strata.proc

---@class strata.proc
local proc = {}

---@class (private) strata.proc.SpawnOpts

---Spawn a command
---@param cmd string|string[]
---@param opts? strata.proc.SpawnOpts
---@return strata.proc.Child
function proc.spawn(cmd, opts) end

---@class strata.proc.Child
local child = {}

---@param cb fun(line: string)
---@return strata.proc.Child
function child:on_line_stdout(cb) end

---@param cb fun(line: string)
---@return strata.proc.Child
function child:on_line_stderr(cb) end

---@param cb fun(exit_code: integer, exit_signal: integer)
---@return strata.proc.Child
function child:on_exit(cb) end

---@return integer exit_code the process' exit code
function child:wait() end

---kill the process
function child:kill() end

