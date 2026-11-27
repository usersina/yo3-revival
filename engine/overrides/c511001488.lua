--yo3-revival override: ProjectIgnis' script, remembering in the activation's label the player the damage (or gain) is for.
--Upstream stored its amount (cv), then read that number as a player in the operation: only an amount of 0 or 1 reversed the
--damage for anyone, and 800 damage to its own controller was reversed for the opponent (batch 24's fuzz: activated 0).
--The label is set only once a chain link has damage or recover info (a nil label was a Lua error: batch 26's fuzz).
--カオス・ライフ
--Chaos Life
local s,id=GetID()
function s.initial_effect(c)
	--reflect
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_CHAINING)
	e1:SetCondition(s.condition)
	e1:SetOperation(s.operation)
	c:RegisterEffect(e1)
end
function s.condition(e,tp,eg,ep,ev,re,r,rp)
	local ex,cg,ct,cp,cv=Duel.GetOperationInfo(ev,CATEGORY_DAMAGE)
	if not ex then
		ex,cg,ct,cp,cv=Duel.GetOperationInfo(ev,CATEGORY_RECOVER)
		if not ex or not Duel.IsPlayerAffectedByEffect(tp,EFFECT_REVERSE_RECOVER) then return false end
		e:SetLabel(cp)
		if cp==tp and cv>=Duel.GetLP(tp) then return true
		elseif cp~=tp and cv>=Duel.GetLP(1-tp) then return true
		elseif cp==PLAYER_ALL then
			local op=0
			if cv>=Duel.GetLP(1-tp) then
				op=op+1
			end
			if cv>=Duel.GetLP(tp) then
				op=op+2
			end
			if op==3 then
				e:SetLabel(PLAYER_ALL)
			elseif op==2 then
				e:SetLabel(tp)
			elseif op==1 then
				e:SetLabel(1-tp)
			else
				return false
			end
			return true
		end
	else
		e:SetLabel(cp)
		if cp==tp and cv>=Duel.GetLP(tp) then return true
		elseif cp~=tp and cv>=Duel.GetLP(1-tp) then return true
		elseif cp==PLAYER_ALL then
			local op=0
			if cv>=Duel.GetLP(1-tp) then
				op=op+1
			end
			if cv>=Duel.GetLP(tp) then
				op=op+2
			end
			if op==3 then
				e:SetLabel(PLAYER_ALL)
			elseif op==2 then
				e:SetLabel(tp)
			elseif op==1 then
				e:SetLabel(1-tp)
			else
				return false
			end
			return true
		end
	end
	return false
end
function s.operation(e,tp,eg,ep,ev,re,r,rp)
	local cid=Duel.GetChainInfo(ev,CHAININFO_CHAIN_ID)
	local cp=e:GetLabel()
	local e1=Effect.CreateEffect(e:GetHandler())
	e1:SetType(EFFECT_TYPE_FIELD)
	e1:SetCode(EFFECT_REVERSE_DAMAGE)
	e1:SetProperty(EFFECT_FLAG_PLAYER_TARGET)
	if cp==tp then
		e1:SetTargetRange(1,0)
	elseif cp~=tp then
		e1:SetTargetRange(0,1)
	else
		e1:SetTargetRange(1,1)
	end
	e1:SetLabel(cid)
	e1:SetValue(s.refcon)
	e1:SetReset(RESET_CHAIN)
	Duel.RegisterEffect(e1,tp)
end
function s.refcon(e,re,r,rp,rc)
	local cc=Duel.GetCurrentChain()
	if cc==0 or (r&REASON_EFFECT)==0 then return end
	local cid=Duel.GetChainInfo(0,CHAININFO_CHAIN_ID)
	return cid==e:GetLabel()
end