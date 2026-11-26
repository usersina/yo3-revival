--Limit Tribute
--Override (engine/overrides/README.md): "only Tribute 1 monster per turn" also stops a Summon that needs 2 or more Tributes
--at once; upstream counted a Tribute only after it happened, so such a Summon went through (scenario, 2026-11-26).
local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_FREE_CHAIN)
	e1:SetHintTiming(0,TIMING_DRAW_PHASE)
	c:RegisterEffect(e1)
	--Restrict Release
	local e2=Effect.CreateEffect(c)
	e2:SetType(EFFECT_TYPE_FIELD)
	e2:SetRange(LOCATION_SZONE)
	e2:SetCode(EFFECT_CANNOT_RELEASE)
	e2:SetProperty(EFFECT_FLAG_PLAYER_TARGET)
	e2:SetTargetRange(1,0)
	e2:SetCondition(s.con)
	e2:SetLabel(0)
	c:RegisterEffect(e2)
	local e3=e2:Clone()
	e3:SetTargetRange(0,1)
	e3:SetLabel(1)
	e3:SetCondition(s.con)
	c:RegisterEffect(e3)
	--Responding
	local e4=Effect.CreateEffect(c)
	e4:SetType(EFFECT_TYPE_CONTINUOUS+EFFECT_TYPE_FIELD)
	e4:SetCode(EVENT_RELEASE)
	e4:SetRange(LOCATION_SZONE)
	e4:SetCondition(s.regcon)
	e4:SetOperation(s.regop)
	c:RegisterEffect(e4)
	--A Summon needing 2 or more Tributes needs more than the 1 Tribute a turn allows
	local e5=Effect.CreateEffect(c)
	e5:SetType(EFFECT_TYPE_FIELD)
	e5:SetRange(LOCATION_SZONE)
	e5:SetCode(EFFECT_CANNOT_SUMMON)
	e5:SetProperty(EFFECT_FLAG_PLAYER_TARGET)
	e5:SetTargetRange(1,1)
	e5:SetTarget(function(e,mc) return (mc:GetTributeRequirement() or 0)>1 end)
	c:RegisterEffect(e5)
	local e6=e5:Clone()
	e6:SetCode(EFFECT_CANNOT_MSET)
	c:RegisterEffect(e6)
end
function s.con(e)
	local p
	if e:GetLabel()==0 then p=e:GetHandlerPlayer() else p=1-e:GetHandlerPlayer() end
	return e:GetHandler():GetFlagEffect(id+p)~=0
end
function s.regcon(e,tp,eg,ep,ev,re,r,rp)
	return eg:IsExists(Card.IsMonster,1,nil)
end
function s.regop(e,tp,eg,ep,ev,re,r,rp)
	e:GetHandler():RegisterFlagEffect(id+rp,RESET_EVENT+RESETS_STANDARD+RESET_PHASE+PHASE_END,0,1)
end