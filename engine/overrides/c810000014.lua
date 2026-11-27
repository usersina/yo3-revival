--The Legendary Gambler
--yo3-revival override: ProjectIgnis' script, answering only the opponent's die roll for an effect in a chain, once per
--chain link. Upstream answered every roll, its own too, so its own roll started it again (the core's "recursive event
--trigger") and a roll outside a chain left it no effect to negate (batch 4's fuzz).
local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_FREE_CHAIN)
	c:RegisterEffect(e1)
	--When your opponent rolls a six-sided die to activate a card or effect, you can roll a six-sided die. If your die
	--result is equal to or higher than your opponent's, negate your opponent's card effect.
	local e2=Effect.CreateEffect(c)
	e2:SetCategory(CATEGORY_NEGATE)
	e2:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
	e2:SetCode(EVENT_TOSS_DICE_NEGATE)
	e2:SetRange(LOCATION_SZONE)
	e2:SetCondition(s.condition)
	e2:SetOperation(s.activate)
	c:RegisterEffect(e2)
end
s.roll_dice=true
function s.condition(e,tp,eg,ep,ev,re,r,rp)
	local cc=Duel.GetCurrentChain()
	if cc==0 or ep==tp then return false end
	return s[0]~=Duel.GetChainInfo(cc,CHAININFO_CHAIN_ID) and Duel.IsChainDisablable(cc)
end
function s.activate(e,tp,eg,ep,ev,re,r,rp)
	local cc=Duel.GetCurrentChain()
	s[0]=Duel.GetChainInfo(cc,CHAININFO_CHAIN_ID)
	if not Duel.SelectYesNo(tp,aux.Stringid(id,0)) then return end
	Duel.Hint(HINT_CARD,0,id)
	local opponent=Duel.GetDiceResult()
	if Duel.TossDice(tp,1)>=opponent then
		Duel.NegateEffect(cc)
	end
end
