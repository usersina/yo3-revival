--yo3-revival override: ProjectIgnis' script, with the Adjust effect (and its Summon-event twins) made safe against itself. The
--destruction down to one monster asks the player to choose and then Destroys: the core raises EVENT_ADJUST again in the middle
--of that, while the chosen monster is still on the field, so the condition held again and the same operation was started a
--second time inside the first ("recursive event trigger detected", found by the fuzz of Celebration of Creation, seed 59650:
--a Set Dark the Hidden Knight chosen as the monster to destroy). A flag on the card now makes the condition false while the
--operation runs; the operation itself still destroys every player's extra monsters, so the text is kept.
--狭小の地下道
--Narrow Tunnel
--scripted by UnknownGuest
local s,id=GetID()
function s.initial_effect(c)
	-- Activate
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_FREE_CHAIN)
	c:RegisterEffect(e1)
	--Each player can only Summon and control up to 1 monster
	local e2=Effect.CreateEffect(c)
	e2:SetType(EFFECT_TYPE_FIELD)
	e2:SetRange(LOCATION_SZONE)
	e2:SetCode(EFFECT_CANNOT_SPECIAL_SUMMON)
	e2:SetProperty(EFFECT_FLAG_PLAYER_TARGET)
	e2:SetTargetRange(1,1)
	e2:SetTarget(s.sumlimit)
	c:RegisterEffect(e2)
	local e3=e2:Clone()
	e3:SetCode(EFFECT_CANNOT_SUMMON)
	c:RegisterEffect(e3)
	local e4=e2:Clone()
	e4:SetCode(EFFECT_CANNOT_FLIP_SUMMON)
	c:RegisterEffect(e4)
	--Destroy monsters until a player controls only 1
	local e8=Effect.CreateEffect(c)
	e8:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
	e8:SetProperty(EFFECT_FLAG_IGNORE_IMMUNE)
	e8:SetCode(EVENT_MSET)
	e8:SetRange(LOCATION_SZONE)
	e8:SetCondition(s.descon)
	e8:SetOperation(s.desop)
	c:RegisterEffect(e8)
	local e9=e8:Clone()
	e9:SetCode(EVENT_SPSUMMON_SUCCESS)
	c:RegisterEffect(e9)
	local e10=e8:Clone()
	e10:SetCode(EVENT_SUMMON_SUCCESS)
	c:RegisterEffect(e10)
	local e11=e8:Clone()
	e11:SetCode(EVENT_FLIP_SUMMON_SUCCESS)
	c:RegisterEffect(e11)
	--Adjust
	local e12=Effect.CreateEffect(c)
	e12:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
	e12:SetProperty(EFFECT_FLAG_IGNORE_IMMUNE)
	e12:SetCode(EVENT_ADJUST)
	e12:SetRange(LOCATION_SZONE)
	e12:SetCondition(s.descon)
	e12:SetOperation(s.desop)
	c:RegisterEffect(e12)
end
function s.sumlimit(e,c,sump,sumtype,sumpos,targetp)
	local p=targetp or sump
	return Duel.GetActivityCount(p,ACTIVITY_SUMMON)>0
		or Duel.GetActivityCount(p,ACTIVITY_SPSUMMON)>0
		or Duel.GetActivityCount(p,ACTIVITY_FLIPSUMMON)>0
		or Duel.GetActivityCount(p,ACTIVITY_NORMALSUMMON)>0
end
function s.descon(e,tp,eg,ep,ev,re,r,rp)
	if e:GetHandler():HasFlagEffect(id) then return false end
	return Duel.GetFieldGroupCount(tp,LOCATION_MZONE,0)>=2 or Duel.GetFieldGroupCount(tp,0,LOCATION_MZONE)>=2
end
function s.desop(e,tp,eg,ep,ev,re,r,rp)
	local c=e:GetHandler()
	c:RegisterFlagEffect(id,RESET_EVENT|RESETS_STANDARD|RESET_PHASE|PHASE_END,0,1)
	for p=0,1 do
		local ft=Duel.GetFieldGroupCount(p,LOCATION_MZONE,0,nil)
		if ft>1 then
			local ct=ft-1
			Duel.Hint(HINT_SELECTMSG,p,HINTMSG_DESTROY)
			local g=Duel.SelectMatchingCard(p,nil,p,LOCATION_MZONE,0,ct,ct,nil)
			Duel.Destroy(g,REASON_EFFECT)
		end
	end
	c:ResetFlagEffect(id)
end