--yo3-revival override: ProjectIgnis' script, whose condition read `A and (hand==1 and a Meklord in the hand) or hand==0`: played
--from the hand it could never be activated (the hand holds this card, and a Meklord card besides), and Set it asked for no
--Meklord monster at all. The card's text is "If you control a "Meklord" monster and no other cards in your hand: Draw 2 cards."
--オーロラ・ドロー
--Aurora Draw
local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetCategory(CATEGORY_DRAW)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetProperty(EFFECT_FLAG_PLAYER_TARGET)
	e1:SetCode(EVENT_FREE_CHAIN)
	e1:SetCondition(s.ntcon)
	e1:SetTarget(s.target)
	e1:SetOperation(s.activate)
	c:RegisterEffect(e1)
end
function s.cfilter(c)
	return c:GetCode()==id
end
function s.ntcon(e,tp,eg,ep,ev,re,r,rp)
	local c=e:GetHandler()
	if c==nil then return true end
	--the card's text: a "Meklord" monster you control and no other cards in your hand (this card, played from the hand, is no "other")
	local others=Duel.GetFieldGroupCount(tp,LOCATION_HAND,0)
	if c:IsLocation(LOCATION_HAND) then others=others-1 end
	return others==0 and Duel.IsExistingMatchingCard(aux.FaceupFilter(Card.IsSetCard,SET_MEKLORD),tp,LOCATION_MZONE,0,1,nil)
end
function s.target(e,tp,eg,ep,ev,re,r,rp,chk)
	if chk==0 then return Duel.IsPlayerCanDraw(tp,2) end
	Duel.SetTargetPlayer(tp)
	Duel.SetTargetParam(2)
	Duel.SetOperationInfo(0,CATEGORY_DRAW,nil,0,tp,2)
end
function s.activate(e,tp,eg,ep,ev,re,r,rp)
	local p,d=Duel.GetChainInfo(0,CHAININFO_TARGET_PLAYER,CHAININFO_TARGET_PARAM)
	Duel.Draw(p,d,REASON_EFFECT)
end