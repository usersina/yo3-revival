--Ritual of the Matador
--yo3-revival override: ProjectIgnis' script, bringing the game's Matador Archfiend (7622360) as well. Upstream knew only the
--anime's own "Archfiend Matador" (511000009), a card the game does not have, so the Ritual Spell could never Ritual Summon
--anything here (batch 31's fuzz: never activated). The game's Matador Archfiend is the same card, by its printed name.
local s,id=GetID()
function s.initial_effect(c)
	Ritual.AddProcGreaterCode(c,6,nil,511000009,7622360)
end
