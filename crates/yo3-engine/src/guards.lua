-- YO3 Revival's guards, loaded after utility.lua into every duel and the bot's look-ahead copies, whatever the house
-- rules (docs/anime-cards.md, the Gods' manga versions).
--
-- Infinity is one value. The scripts give an infinite ATK as 9,999,999 (the manga Obelisk's Soul Energy Max, the anime
-- Divine Serpent Geh, Avatar matching them) and their own infinite damage as "100 times the LP"; any LP, ATK or DEF of
-- 9,999,999 or more is ∞, and becomes exactly INFINITE, whatever made it: Ra fed with ∞ LP, Riryoku's half of ∞ LP,
-- Mystik Wok on an ∞ ATK. INFINITE is twice the threshold, so half of ∞ is still ∞, and far enough below 32 bits for the
-- core's sums of ATK (int32, no ceiling) and for scripts that build a list as long as the LP (Bark of Dark Ruler).
--   LP: a finite cost, damage or gain leaves ∞ LP as it is (as against 0 ATK: no change, no event). An amount that is
--   itself ∞ moves as is: Ra's "pay all but 1" leaves 1 and gives Ra ∞, De-Fusion gives ∞ back, Solemn Judgment's half
--   of ∞ leaves ∞. "LP becomes X" (Life Equalizer, Rasca, Self-Destruct Button) is X; Yu-Jo Friendship's half of ∞ plus
--   anything is ∞ for both.
--   ATK and DEF: -500, half or double of ∞ is ∞; ∞ against ∞ is a tie. "ATK becomes X" is X.
--   Damage that is ∞ (an amount of 9,999,999 or more, or battle damage from a monster with ∞ ATK, directly or through a
--   monster with finite ATK) takes all of a player's LP, ∞ LP too: Ring of Destruction on Geh is a draw.
--
-- 32 bits. The core takes LP amounts as 32-bit integers, so a larger number wraps around: "100 times the LP" past
-- 21,474,836 LP became a gain of over a billion LP. Every damage and LP gain is kept within what the core can hold.
local MAX = 0x7fffffff
local INFINITY = 9999999
local INFINITE = 2 * INFINITY
local function capped(v) return (type(v) == "number" and v > MAX) and MAX or v end
local function infinite(v) return (type(v) == "number" and v >= INFINITY) and MAX or v end
local function finite(v) return type(v) == "number" and v < INFINITY end
local function infinite_lp(p) return Duel.GetLP(p) >= INFINITY end
local damage, recover, battle, setlp, paylp = Duel.Damage, Duel.Recover, Duel.ChangeBattleDamage, Duel.SetLP, Duel.PayLPCost
-- An LP of ∞ or more becomes exactly ∞.
local function settle(p)
	local lp = Duel.GetLP(p)
	if lp >= INFINITY and lp ~= INFINITE then setlp(p, INFINITE) end
end
Duel.Damage = function(p, v, ...)
	v = capped(infinite(v))
	if finite(v) and infinite_lp(p) then v = 0 end
	return damage(p, v, ...)
end
Duel.ChangeBattleDamage = function(p, v, ...)
	v = capped(infinite(v))
	if finite(v) and infinite_lp(p) then v = 0 end
	return battle(p, v, ...)
end
Duel.Recover = function(p, v, ...)
	if finite(v) and infinite_lp(p) then return 0 end
	if type(v) == "number" then v = math.min(v, MAX - Duel.GetLP(p)) end
	local gained = recover(p, v, ...)
	settle(p)
	return gained
end
Duel.PayLPCost = function(p, v, ...)
	if finite(v) and infinite_lp(p) then return end
	paylp(p, v, ...)
	settle(p)
end
Duel.SetLP = function(p, v, ...)
	if type(v) == "number" and v >= INFINITY then v = INFINITE end
	return setlp(p, capped(v), ...)
end
-- LP the core changes by itself (damage turned into a gain, a duel started with more): settled too.
for _, code in ipairs({EVENT_RECOVER, EVENT_ADJUST}) do
	local settling = Effect.GlobalEffect()
	settling:SetType(EFFECT_TYPE_FIELD + EFFECT_TYPE_CONTINUOUS)
	settling:SetCode(code)
	settling:SetProperty(EFFECT_FLAG_CANNOT_DISABLE + EFFECT_FLAG_IGNORE_IMMUNE)
	settling:SetOperation(function() settle(0) settle(1) end)
	Duel.RegisterEffect(settling, 0)
end
-- Battle damage the core works out itself: from a face-up Attack Position monster with infinite ATK, all of it; any
-- other to a player with ∞ LP, none.
local infinite_battle = Effect.GlobalEffect()
infinite_battle:SetType(EFFECT_TYPE_FIELD + EFFECT_TYPE_CONTINUOUS)
infinite_battle:SetCode(EVENT_PRE_BATTLE_DAMAGE)
infinite_battle:SetOperation(function(e, tp, eg, ep, ev, re, r, rp)
	local attacker, target = Duel.GetAttacker(), Duel.GetAttackTarget()
	local from = (attacker and attacker:IsControler(1 - ep)) and attacker or target
	if ev > 0 and from and from:IsFaceup() and from:IsAttackPos() and from:GetAttack() >= INFINITY then
		battle(ep, MAX)
	elseif ev > 0 and infinite_lp(ep) then
		battle(ep, 0)
	end
end)
Duel.RegisterEffect(infinite_battle, 0)
-- ATK and DEF of ∞ or more become exactly ∞: a final change applied after every other (the core gives a delayed final
-- value its card's ATK as worked out so far, card::get_attack), whatever the card is immune to. An ATK below ∞ is left as
-- it is.
for _, code in ipairs({EFFECT_SET_ATTACK_FINAL, EFFECT_SET_DEFENSE_FINAL}) do
	local stat = code == EFFECT_SET_ATTACK_FINAL and Card.GetAttack or Card.GetDefense
	local one = Effect.GlobalEffect()
	one:SetType(EFFECT_TYPE_FIELD)
	one:SetCode(code)
	one:SetProperty(EFFECT_FLAG_CANNOT_DISABLE + EFFECT_FLAG_IGNORE_IMMUNE + EFFECT_FLAG_DELAY)
	one:SetTargetRange(LOCATION_MZONE, LOCATION_MZONE)
	one:SetValue(function(e, c)
		local v = stat(c)
		return v >= INFINITY and INFINITE or v
	end)
	Duel.RegisterEffect(one, 0)
end

-- Number declarations. The core sends a declaration's options with their count in one byte (playerop.cpp,
-- MSG_ANNOUNCE_NUMBER), so a longer list reached the player cut to its count modulo 256: Wall of Revealing Light at ∞ LP
-- offered 1000 to 23,000, Bark of Dark Ruler from 25,600 LP, and none at all at a multiple of 256. A list longer than 255
-- keeps its first 200 options and 55 more spread over the rest, ending with its last; the index returned is the one in the
-- script's own list.
local ANNOUNCE_LIMIT, ANNOUNCE_HEAD = 255, 200
local function shortened(n)
	local picks = {}
	for i = 1, ANNOUNCE_HEAD do picks[i] = i end
	local rest = ANNOUNCE_LIMIT - ANNOUNCE_HEAD
	for k = 1, rest do picks[ANNOUNCE_HEAD + k] = ANNOUNCE_HEAD + math.ceil(k * (n - ANNOUNCE_HEAD) / rest) end
	return picks
end
local announce, announce_range = Duel.AnnounceNumber, Duel.AnnounceNumberRange
Duel.AnnounceNumber = function(p, ...)
	local options = {...}
	if #options == 1 and type(options[1]) == "table" then options = options[1] end
	if #options <= ANNOUNCE_LIMIT then return announce(p, ...) end
	local picks, kept = shortened(#options), {}
	for i, at in ipairs(picks) do kept[i] = options[at] end
	local value, index = announce(p, table.unpack(kept))
	return value, picks[index + 1] - 1
end
Duel.AnnounceNumberRange = function(p, low, high, ...)
	if type(low) ~= "number" or type(high) ~= "number" or math.abs(high - low) < ANNOUNCE_LIMIT then
		return announce_range(p, low, high, ...)
	end
	if low > high then low, high = high, low end
	local excluded = {}
	for _, v in ipairs({...}) do if type(v) == "number" then excluded[v] = true end end
	if select("#", ...) == 1 and type(...) == "table" then for _, v in ipairs(...) do excluded[v] = true end end
	local options = {}
	for v = low, high do if not excluded[v] then options[#options + 1] = v end end
	if #options <= ANNOUNCE_LIMIT then return announce(p, table.unpack(options)) end
	local picks, kept = shortened(#options), {}
	for i, at in ipairs(picks) do kept[i] = options[at] end
	local value, index = announce(p, table.unpack(kept))
	return value, picks[index + 1] - 1
end
Duel.AnnounceLevel = Duel.AnnounceNumberRange

-- A printed infinity. A card whose printed ATK is infinite (in the anime: "ATTACK ∞") has it as its ATK, not as an
-- effect: the scripts can only give it as ATK "?" and an effect that makes it 9,999,999, which Skill Drain would negate
-- and other cards would raise, halve or swap. So a card on this list, while in a Monster Zone, keeps its ATK at
-- ∞ whatever negates its effects (its other effects are still negated), and is unaffected by other cards' effects
-- that change its ATK; its original ATK is ∞ too. Its DEF is an ordinary number. Cards that become infinite
-- through an effect (Obelisk's Soul Energy Max) are not on the list: docs/added-cards.md, "Infinite ATK".
local PRINTED_INFINITY = {
	[170000170] = true, -- Divine Serpent Geh (Anime)
}
local ATK_CHANGES = {}
for _, name in ipairs({"EFFECT_UPDATE_ATTACK", "EFFECT_SET_ATTACK", "EFFECT_SET_ATTACK_FINAL", "EFFECT_SET_BASE_ATTACK",
		"EFFECT_SET_BASE_ATTACK_FINAL", "EFFECT_SWAP_AD", "EFFECT_SWAP_BASE_AD"}) do
	if _G[name] then ATK_CHANGES[_G[name]] = true end
end
local PRINTED_FLAG = 830000002
local function printed_infinity(c)
	return PRINTED_INFINITY[c:GetOriginalCode()] and c:GetFlagEffect(PRINTED_FLAG) == 0
end
local printed = Effect.GlobalEffect()
printed:SetType(EFFECT_TYPE_FIELD + EFFECT_TYPE_CONTINUOUS)
printed:SetCode(EVENT_ADJUST)
printed:SetProperty(EFFECT_FLAG_CANNOT_DISABLE + EFFECT_FLAG_IGNORE_IMMUNE)
printed:SetOperation(function()
	for c in aux.Next(Duel.GetMatchingGroup(printed_infinity, 0, LOCATION_MZONE, LOCATION_MZONE, nil)) do
		c:RegisterFlagEffect(PRINTED_FLAG, RESET_EVENT + RESETS_STANDARD, 0, 1)
		local atk = Effect.CreateEffect(c)
		atk:SetType(EFFECT_TYPE_SINGLE)
		atk:SetProperty(EFFECT_FLAG_SINGLE_RANGE + EFFECT_FLAG_CANNOT_DISABLE + EFFECT_FLAG_UNCOPYABLE)
		atk:SetRange(LOCATION_MZONE)
		atk:SetCode(EFFECT_SET_ATTACK_FINAL)
		atk:SetValue(INFINITE)
		atk:SetReset(RESET_EVENT + RESETS_STANDARD)
		c:RegisterEffect(atk, true)
		-- Its original ATK too, as printed: the field shows it white (equal to the original), an effect's infinity blue.
		local original = atk:Clone()
		original:SetCode(EFFECT_SET_BASE_ATTACK)
		c:RegisterEffect(original, true)
		local unaffected = atk:Clone()
		unaffected:SetCode(EFFECT_IMMUNE_EFFECT)
		unaffected:SetValue(function(e, te) return te:GetOwner() ~= e:GetHandler() and ATK_CHANGES[te:GetCode()] == true end)
		c:RegisterEffect(unaffected, true)
	end
end)
Duel.RegisterEffect(printed, 0)
