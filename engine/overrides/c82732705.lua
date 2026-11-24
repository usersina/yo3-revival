--スキルドレイン
--Skill Drain
--yo3-revival override, for the Gods as the `anime-cards` setting plays them: the manga Egyptian Gods (Obelisk the
--Tormentor, Slifer the Sky Dragon, The Winged Dragon of Ra: 513000135/6/4) and the manga Wicked Gods (Dreadroot, Avatar,
--Eraser: 513000137/8/9), whose text says "Other cards' effects only apply to this card for 1 turn", and the anime Gods'
--scripts (511600398/9/400) the map played before. Skill Drain against any other monster is ProjectIgnis' script,
--unchanged. The rule for the Gods (#60):
--  * a God already on the field when Skill Drain is activated has its effects negated at once, until the end of that turn;
--    its effects come back when the turn ends, though Skill Drain stays face-up (Ra's ATK, the Tributes' total, is 0 for
--    the rest of that turn, and the total again from the next one: its effect that sets it is negated like the rest);
--  * a God that arrives while Skill Drain is already face-up is not affected at all;
--  * each activation counts on its own (the anime's rule for the Gods: "activated cards or effects can only affect this card
--    until the End Phase"): a second copy of Skill Drain, or the same card activated again, negates the Gods on the field
--    at that moment until the end of that turn, even one an earlier activation already negated
--    (tests/anime_reconstructions.rs `a_second_skill_drain_negates_a_god_again_for_its_turn`);
--  * a God that leaves the field (or is turned face-down) and comes back is a new card to the game: it is not affected by the
--    Skill Drain already face-up, and is negated by the next activation (`a_god_that_comes_back_is_a_new_card`).
--When an activation of Skill Drain resolves, each God on the field gets a "negated" flag that ends with the turn. Only a God
--with that flag is negated by Skill Drain. The recovery at the end of the turn is the engine's reset of that flag, not an effect of the God, so no negation
--(Skill Drain's own included) can keep it from coming back.
local s,id=GetID()
s.gods={511600398,511600399,511600400,513000134,513000135,513000136,513000137,513000138,513000139}
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
		if tc:GetFlagEffect(s.negated)==0 then
			tc:RegisterFlagEffect(s.negated,RESET_EVENT|RESETS_STANDARD|RESET_PHASE|PHASE_END,0,1)
		end
	end
end
function s.disable(e,c)
	if s.isgod(c) then return c:GetFlagEffect(s.negated)>0 end
	return c:IsType(TYPE_EFFECT) or (c:GetOriginalType()&TYPE_EFFECT)==TYPE_EFFECT
end
