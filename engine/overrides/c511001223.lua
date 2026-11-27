--Spell Spice Cinnamon
--yo3-revival override: ProjectIgnis' script, which indexes the targets its effect chose without checking there are
--any. Spell Transfer, negating Cinnamon, runs its target step and then its operation as its own, for the controller
--of Spell Transfer, who may have no Attack Position monster to choose or no Defense Position monster of the opponent
--to pair with it: the label object was nil and the operation raised a Lua error (batch 15's fuzz, seed 64067). With
--no pair of targets the effect now does nothing.
local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetCategory(CATEGORY_POSITION)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetProperty(EFFECT_FLAG_CARD_TARGET)
	e1:SetCode(EVENT_FREE_CHAIN)
	e1:SetTarget(s.target)
	e1:SetOperation(s.activate)
	c:RegisterEffect(e1)
end
function s.filter1(c)
	return c:IsFaceup() and c:IsPosition(POS_FACEUP_ATTACK)
end
function s.filter2(c)
	return c:IsFaceup() and c:IsPosition(POS_FACEUP_DEFENSE)
end
function s.target(e,tp,eg,ep,ev,re,r,rp,chk,chkc)
	if chkc then return false end
	if chk==0 then return Duel.IsExistingTarget(s.filter1,tp,LOCATION_MZONE,0,1,nil)
		and Duel.IsExistingTarget(s.filter2,tp,0,LOCATION_MZONE,1,nil) end
	Duel.Hint(HINT_SELECTMSG,tp,HINTMSG_POSCHANGE)
	local g1=Duel.SelectTarget(tp,s.filter1,tp,LOCATION_MZONE,0,1,1,nil)
	e:SetLabelObject(g1:GetFirst())
	Duel.Hint(HINT_SELECTMSG,tp,HINTMSG_POSCHANGE)
	local g2=Duel.SelectTarget(tp,s.filter2,tp,0,LOCATION_MZONE,1,1,nil)
	Duel.SetOperationInfo(0,CATEGORY_POSITION,g1,1,0,0)
	Duel.SetOperationInfo(0,CATEGORY_POSITION,g2,1,0,0)
end
function s.activate(e,tp,eg,ep,ev,re,r,rp)
	local tc1=e:GetLabelObject()
	local g=Duel.GetChainInfo(0,CHAININFO_TARGET_CARDS)
	if not tc1 or not g then return end
	local tc2=g:GetFirst()
	if tc2==tc1 then tc2=g:GetNext() end
	if not tc2 then return end
	if tc1:IsRelateToEffect(e) and tc1:IsFaceup() and tc2:IsRelateToEffect(e) then 
		Duel.ChangePosition(tc1,POS_FACEUP_DEFENSE)
		Duel.ChangePosition(tc2,POS_FACEUP_ATTACK)
	end
end
