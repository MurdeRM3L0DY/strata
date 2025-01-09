local ks = strata.input.Keys
local ms = strata.input.Modifiers

strata.input.setup {
	repeat_info = {
		rate = 30,
		delay = 150,
	},

	xkbconfig = {
		layout = "it",
		rules = "",
		model = "",
		options = "caps:swapescape",
		variant = "",
	},
}

strata.input.keybind(ms.Control_L + ms.Alt_L, ks.Return, function()
	print("spawning kitty")
	strata.proc.spawn("kitty")
end)

strata.input.keybind(ms.Control_L + ms.Alt_L, ks.Escape, function()
	print("quitting strata")
	strata.quit()
end)

-- alternatives???
-- local pid, stdout, stderr = strata.proc.spawn({ "inotify", "..." })
-- stdout:read(function(...)
-- end)
-- stderr:read(function(...)
-- end)

-- print("(kitty) pid=" .. strata.proc.spawn({ "echo", "hello world" }, {
-- 	stdout = function(out)
-- 		print("echo -> " .. out)
-- 	end
-- }))

-- print("(echo) pid=" .. strata.proc.spawn { "echo", "hello world" })

strata.proc.spawn({ "pactl", "subscribe" }, {
	stdout = function(out) print("(pactl) stdout=" .. out) end,
	stderr = function(err) print("(pactl) stderr=", err) end,
})
