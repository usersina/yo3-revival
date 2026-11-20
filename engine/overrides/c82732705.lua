--スキルドレイン
--Skill Drain
--yo3-revival override, for the anime Egyptian Gods (Obelisk the Tormentor, Slifer the Sky Dragon, The Winged Dragon of Ra:
--the anime scripts 511600398/9/400, played only with the `anime-cards` setting). Skill Drain against any other monster is
--ProjectIgnis' script, unchanged. The rule for the Gods (#60):
--  * a God already on the field when Skill Drain is activated has its effects negated at once, until the end of that turn;
--    its effects come back when the turn ends, though Skill Drain stays face-up (Ra's ATK, the Tributes' total, is 0 for
--    the rest of that turn, and the total again from the next one: its effect that sets it is negated like the rest);
--  * a God that arrives while Skill Drain is already face-up is not affected at all.
--The Gods already on the field are marked when Skill Drain's activation resolves (a flag that ends with the turn, or when
--the God leaves the field or is turned face-down), and only a marked God is negated by Skill Drain.
local s,id=GetID()
s.gods={511600398,511600399,511600400}
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
		tc:RegisterFlagEffect(id,RESET_EVENT|RESETS_STANDARD|RESET_PHASE|PHASE_END,0,1)
	end
end
function s.disable(e,c)
	if s.isgod(c) then return c:GetFlagEffect(id)>0 end
	return c:IsType(TYPE_EFFECT) or (c:GetOriginalType()&TYPE_EFFECT)==TYPE_EFFECT
end
