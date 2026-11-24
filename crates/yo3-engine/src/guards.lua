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
