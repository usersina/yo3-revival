--究極恐獣
--Ultimate Tyranno
--yo3-revival override, from ProjectIgnis' pre-errata/c511003009.lua: "If this card is in face-up Attack Position at
--the beginning of your Battle Phase, it must attack first, and attack all monsters your opponent controls once each"
--(the client's text). Today's text lets it attack all monsters but doesn't make it, and stops your other monsters
--attacking while it can.
local s,id=GetID()
--A flag on the card that attacked first this Battle Phase.
local FLAG_FIRST=id+1
function s.initial_effect(c)
	--attack
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_SINGLE)
	e1:SetCode(EFFECT_FIRST_ATTACK)
	e1:SetCondition(s.facon)
	c:RegisterEffect(e1)
	local e2=Effect.CreateEffect(c)
	e2:SetType(EFFECT_TYPE_SINGLE)
	e2:SetCode(EFFECT_MUST_ATTACK)
	e2:SetCondition(s.facon)
	c:RegisterEffect(e2)
	local e3=Effect.CreateEffect(c)
	e3:SetType(EFFECT_TYPE_SINGLE)
	e3:SetCode(EFFECT_ATTACK_ALL)
	e3:SetCondition(s.facon)
	e3:SetValue(1)
	c:RegisterEffect(e3)
	local e4=Effect.CreateEffect(c)
	e4:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
	e4:SetCode(EVENT_PHASE_START+PHASE_BATTLE_START)
	e4:SetRange(LOCATION_MZONE)
	e4:SetOperation(s.regop)
	c:RegisterEffect(e4)
	aux.GlobalCheck(s,function()
		s[0]=0
		s[1]=0
		local ge1=Effect.CreateEffect(c)
		ge1:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
		ge1:SetCode(EVENT_ATTACK_ANNOUNCE)
		ge1:SetOperation(s.checkop)
		Duel.RegisterEffect(ge1,0)
		local ge2=Effect.CreateEffect(c)
		ge2:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
		ge2:SetCode(EVENT_PHASE_START+PHASE_BATTLE_START)
		ge2:SetOperation(s.clear)
		Duel.RegisterEffect(ge2,0)
	end)
end
--s[p]: 0 before player p's first attack this Battle Phase, 1 after it, 2 once another monster attacked
function s.checkop(e,tp,eg,ep,ev,re,r,rp)
	local tc=eg:GetFirst()
	if s[tc:GetControler()]==0 then
		s[tc:GetControler()]=1
		tc:RegisterFlagEffect(FLAG_FIRST,RESET_EVENT+RESETS_STANDARD+RESET_PHASE+PHASE_BATTLE,0,1)
	elseif tc:GetFlagEffect(FLAG_FIRST)==0 then
		s[tc:GetControler()]=2
	end
end
function s.clear(e,tp,eg,ep,ev,re,r,rp)
	s[0]=0
	s[1]=0
end
function s.facon(e)
	local tp=e:GetHandlerPlayer()
	return Duel.GetFieldGroupCount(tp,0,LOCATION_MZONE)>0
		and e:GetHandler():GetFlagEffect(id)>0
		and s[tp]==e:GetHandler():GetFlagEffect(FLAG_FIRST)
end
function s.regop(e,tp,eg,ep,ev,re,r,rp)
	local c=e:GetHandler()
	if Duel.IsTurnPlayer(tp) and c:IsPosition(POS_FACEUP_ATTACK) then
		c:RegisterFlagEffect(id,RESET_EVENT+RESETS_STANDARD+RESET_PHASE+PHASE_BATTLE,0,1)
	end
end
