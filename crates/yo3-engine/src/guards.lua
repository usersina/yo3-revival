-- YO3 Revival's guards, loaded after utility.lua into every duel and the bot's look-ahead copies, whatever the house
-- rules: not rules, fixes. The core takes LP amounts as 32-bit integers, so a larger number wraps around: the manga
-- Obelisk's Soul Energy Max and the anime Divine Serpent Geh deal "the opponent's LP x 100", which past 21,474,836 LP
-- (three Mystik Woks on an infinite ATK) became a gain of over a billion LP. Every damage and LP gain is kept within
-- what the core can hold: an amount above it still takes all of a player's LP.
local MAX = 0x7fffffff
local function capped(v) return (type(v) == "number" and v > MAX) and MAX or v end
local damage, recover, battle = Duel.Damage, Duel.Recover, Duel.ChangeBattleDamage
Duel.Damage = function(p, v, ...) return damage(p, capped(v), ...) end
Duel.ChangeBattleDamage = function(p, v, ...) return battle(p, capped(v), ...) end
Duel.Recover = function(p, v, ...)
	if type(v) == "number" then v = math.min(v, MAX - Duel.GetLP(p)) end
	return recover(p, v, ...)
end
