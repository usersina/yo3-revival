--スキルドレイン
--Skill Drain
--yo3-revival override, for the anime Egyptian Gods (Obelisk the Tormentor, Slifer the Sky Dragon, The Winged Dragon of Ra:
--the anime scripts 511600398/9/400, played only with the `anime-cards` setting). Skill Drain against any other monster is
--ProjectIgnis' script, unchanged. The rule for the Gods (#60):
--  * a God already on the field when Skill Drain is activated has its effects negated at once, until the end of that turn;
--    its effects come back when the turn ends, though Skill Drain stays face-up (Ra's ATK, the Tributes' total, is 0 for
--    the rest of that turn, and the total again from the next one: its effect that sets it is negated like the rest);
--  * a God that arrives while Skill Drain is already face-up is not affected at all;
--  * Skill Drain affects a God once: "until the end of the turn in which it FIRST affects that God". A second copy of
--    Skill Drain activated in a later turn does not negate a God the first one already affected (decision, tests/
--    anime_reconstructions.rs `a_second_skill_drain_does_not_affect_a_god_again`); it does affect a God that Skill Drain
--    has not affected yet (one that arrived after the first activation);
--  * a God that leaves the field (or is turned face-down) and comes back is a new card to the game: it is not affected again
--    by the Skill Drain already face-up, and is affected once by the next one activated (`a_god_that_comes_back_is_a_new_card`).
--The Gods on the field are marked when Skill Drain's activation resolves with two flags on the God: "affected" (kept until the
--God leaves the field or is turned face-down) and "negated" (ends with the turn). Only a God with the "negated" flag is negated by
--Skill Drain. The recovery at the end of the turn is the engine's reset of that flag, not an effect of the God, so no negation
--(Skill Drain's own included) can keep it from coming back.
local s,id=GetID()
s.gods={511600398,511600399,511600400}
s.negated=id+(1<<27)
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_FREE_CHAIN)
	e1:SetCost(Cost.PayLP(1000))
	e1:SetOperation(s.markop)
	c:RegisterEffect(e1)
	--disable
	local e2=Effect.CreateEffect(c)
	e2:SetType(EFFECT_TYPE_FIELD)
	e2:SetRange(LOCATION_SZONE)
	e2:SetTargetRange(LOCATION_MZONE,LOCATION_MZONE)
	e2:SetTarget(s.disable)
	e2:SetCode(EFFECT_DISABLE)
	c:RegisterEffect(e2)
end
function s.isgod(c)
	return c:IsOriginalCode(table.unpack(s.gods))
end
function s.markop(e,tp,eg,ep,ev,re,r,rp)
	local g=Duel.GetMatchingGroup(s.isgod,tp,LOCATION_MZONE,LOCATION_MZONE,nil)
	for tc in g:Iter() do
		if tc:GetFlagEffect(id)==0 then
			tc:RegisterFlagEffect(id,RESET_EVENT|RESETS_STANDARD,0,1)
			tc:RegisterFlagEffect(s.negated,RESET_EVENT|RESETS_STANDARD|RESET_PHASE|PHASE_END,0,1)
		end
	end
end
function s.disable(e,c)
	if s.isgod(c) then return c:GetFlagEffect(s.negated)>0 end
	return c:IsType(TYPE_EFFECT) or (c:GetOriginalType()&TYPE_EFFECT)==TYPE_EFFECT
end
