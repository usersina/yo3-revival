-- YO3 Revival's guards, loaded after utility.lua into every duel and the bot's look-ahead copies, whatever the house
-- rules (docs/anime-cards.md, the Gods' manga versions).
--
-- Infinity. The scripts give an infinite ATK as 9,999,999 (the manga Obelisk's Soul Energy Max, the anime Divine Serpent
-- Geh, Avatar matching them), and their own infinite damage as "100 times the LP": lethal, whatever the LP. Damage that is
-- such an ATK is infinite too, so it takes all of a player's LP, however much Mystik Wok gave them: effect damage of
-- 9,999,999 or more (Ring of Destruction on Geh: a draw), and battle damage dealt by a monster with infinite ATK (Avatar
-- risen to infinity, attacking directly).
--
-- 32 bits. The core takes LP amounts as 32-bit integers, so a larger number wraps around: "100 times the LP" past
-- 21,474,836 LP became a gain of over a billion LP. Every damage and LP gain is kept within what the core can hold.
local MAX = 0x7fffffff
local INFINITY = 9999999
local function capped(v) return (type(v) == "number" and v > MAX) and MAX or v end
local function infinite(v) return (type(v) == "number" and v >= INFINITY) and MAX or v end
local damage, recover, battle = Duel.Damage, Duel.Recover, Duel.ChangeBattleDamage
Duel.Damage = function(p, v, ...) return damage(p, capped(infinite(v)), ...) end
Duel.ChangeBattleDamage = function(p, v, ...) return battle(p, capped(infinite(v)), ...) end
Duel.Recover = function(p, v, ...)
	if type(v) == "number" then v = math.min(v, MAX - Duel.GetLP(p)) end
	return recover(p, v, ...)
end
-- Battle damage the core works out itself: from a face-up Attack Position monster with infinite ATK, all of it.
local infinite_battle = Effect.GlobalEffect()
infinite_battle:SetType(EFFECT_TYPE_FIELD + EFFECT_TYPE_CONTINUOUS)
infinite_battle:SetCode(EVENT_PRE_BATTLE_DAMAGE)
infinite_battle:SetOperation(function(e, tp, eg, ep, ev, re, r, rp)
	local attacker, target = Duel.GetAttacker(), Duel.GetAttackTarget()
	local from = (attacker and attacker:IsControler(1 - ep)) and attacker or target
	if ev > 0 and from and from:IsFaceup() and from:IsAttackPos() and from:GetAttack() >= INFINITY then
		battle(ep, MAX)
	end
end)
Duel.RegisterEffect(infinite_battle, 0)

-- A printed infinity. A card whose printed ATK is infinite (in the anime: "ATTACK ∞") has it as its ATK, not as an
-- effect: the scripts can only give it as ATK "?" and an effect that makes it 9,999,999, which Skill Drain would negate
-- and other cards would raise, halve or swap. So a card on this list, while in a Monster Zone, keeps its ATK at
-- infinity whatever negates its effects (its other effects are still negated), and is unaffected by other cards' effects
-- that change its ATK. Its DEF is an ordinary number. Cards that become infinite through an effect (Obelisk's Soul Energy
-- Max) are not on the list: docs/added-cards.md, "Infinite ATK".
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
		atk:SetValue(INFINITY)
		atk:SetReset(RESET_EVENT + RESETS_STANDARD)
		c:RegisterEffect(atk, true)
		local unaffected = atk:Clone()
		unaffected:SetCode(EFFECT_IMMUNE_EFFECT)
		unaffected:SetValue(function(e, te) return te:GetOwner() ~= e:GetHandler() and ATK_CHANGES[te:GetCode()] == true end)
		c:RegisterEffect(unaffected, true)
	end
end)
Duel.RegisterEffect(printed, 0)
