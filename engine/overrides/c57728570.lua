--死のデッキ破壊ウイルス
--Crush Card Virus
local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetCategory(CATEGORY_DESTROY)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_FREE_CHAIN)
	e1:SetHintTiming(0,TIMING_TOHAND)
	e1:SetCost(s.cost)
	e1:SetTarget(s.target)
	e1:SetOperation(s.activate)
	c:RegisterEffect(e1)
end
function s.costfilter(c)
	return c:IsAttribute(ATTRIBUTE_DARK) and c:IsAttackBelow(1000)
end
function s.cost(e,tp,eg,ep,ev,re,r,rp,chk)
	if chk==0 then return Duel.CheckReleaseGroupCost(tp,s.costfilter,1,false,nil,nil) end
	local g=Duel.SelectReleaseGroupCost(tp,s.costfilter,1,1,false,nil,nil)
	Duel.Release(g,REASON_COST)
end
function s.filter(c)
	return c:IsAttackAbove(1500)
end
function s.hgfilter(c)
	return not c:IsPublic() or s.filter(c)
end
function s.fgfilter(c)
	return c:IsFacedown() or s.filter(c)
end
function s.tgfilter(c)
	return ((c:IsLocation(LOCATION_HAND) and c:IsPublic()) or (c:IsLocation(LOCATION_MZONE) and c:IsFaceup())) and s.filter(c)
end
function s.target(e,tp,eg,ep,ev,re,r,rp,chk)
	if chk==0 then return true end
	local g=Duel.GetMatchingGroup(s.tgfilter,tp,0,LOCATION_MZONE|LOCATION_HAND,nil)
	Duel.SetOperationInfo(0,CATEGORY_DESTROY,g,#g,0,0)
end
--"Check all monsters your opponent controls, your opponent's hand, and all cards they draw (until the end of your
--opponent's 3rd turn after this card's activation), and destroy all monsters with 1500 or more ATK" (the client's text;
--yo3-revival override: today's text checks only the hand and field once, then lets the opponent destroy up to 3 from
--their Deck, and they take no damage for two turns)
function s.activate(e,tp,eg,ep,ev,re,r,rp)
	local conf=Duel.GetFieldGroup(tp,0,LOCATION_MZONE|LOCATION_HAND)
	if #conf>0 then
		Duel.ConfirmCards(tp,conf)
		local dg=conf:Filter(s.filter,nil)
		Duel.Destroy(dg,REASON_EFFECT)
		Duel.ShuffleHand(1-tp)
	end
	if not e:IsHasType(EFFECT_TYPE_ACTIVATE) then return end
	--Their draws until the end of their 3rd turn
	local e1=Effect.CreateEffect(e:GetHandler())
	e1:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
	e1:SetCode(EVENT_DRAW)
	e1:SetOperation(s.drawop)
	e1:SetReset(RESET_PHASE|PHASE_END|RESET_OPPO_TURN,3)
	Duel.RegisterEffect(e1,tp)
end
function s.drawop(e,tp,eg,ep,ev,re,r,rp)
	if ep==tp then return end
	local hg=eg:Filter(Card.IsLocation,nil,LOCATION_HAND)
	if #hg==0 then return end
	Duel.ConfirmCards(tp,hg)
	Duel.Destroy(hg:Filter(s.filter,nil),REASON_EFFECT)
	Duel.ShuffleHand(1-tp)
end
