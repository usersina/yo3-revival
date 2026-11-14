--死皇帝の陵墓
--Mausoleum of the Emperor
local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_FREE_CHAIN)
	c:RegisterEffect(e1)
	--"Both players can Normal Summon or Set monsters without Tributing, by paying 1000 Life Points x the number of
	--monsters needed for the Tribute Summon": a way to Normal Summon or Set, not an effect that activates (the client's
	--text; yo3-revival override: today's text makes it an effect the turn player activates, then summons with)
	for tributes=1,2 do
		local e2=Effect.CreateEffect(c)
		e2:SetDescription(aux.Stringid(id,tributes))
		e2:SetType(EFFECT_TYPE_FIELD)
		e2:SetCode(EFFECT_SUMMON_PROC)
		e2:SetRange(LOCATION_FZONE)
		e2:SetTargetRange(LOCATION_HAND,LOCATION_HAND)
		e2:SetLabel(tributes)
		e2:SetCondition(s.ntcon)
		e2:SetTarget(aux.FieldSummonProcTg(s.nttg))
		e2:SetOperation(s.ntop)
		c:RegisterEffect(e2)
		local e3=e2:Clone()
		e3:SetCode(EFFECT_SET_PROC)
		c:RegisterEffect(e3)
	end
end
function s.ntcon(e,c,minc)
	if c==nil then return true end
	local tp=c:GetControler()
	return minc==0 and Duel.GetLocationCount(tp,LOCATION_MZONE)>0 and Duel.CheckLPCost(tp,e:GetLabel()*1000)
end
function s.nttg(e,c)
	local mi,ma=c:GetTributeRequirement()
	return mi==e:GetLabel() or ma==e:GetLabel()
end
function s.ntop(e,tp,eg,ep,ev,re,r,rp,c)
	Duel.PayLPCost(c:GetControler(),e:GetLabel()*1000)
end
