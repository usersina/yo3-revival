// The core's queries, made safe to ask between two steps of a duel. Compiled with the core by build.rs; core.rs calls
// these instead of OCG_DuelQuery, OCG_DuelQueryLocation and OCG_DuelQueryField.
//
// A step can queue the arguments of its next Lua call and stop there, because it also wrote a message: an extra
// Normal Summon hints the card that grants it, then queues that card's operation with the summoned monster as its
// argument (operations.cpp, the summon's last step). A query asked at that point can call Lua too (a monster's ATK
// from a value function, called with the effect and the card), and that call took the queued argument along with its
// own: "incorrect parameter count", and the operation later ran without its card (issue #9, Brain Research Lab).
// The server asks after every step (the field report, MSG_MOVE's reasons, the bot's look-ahead), so each query runs
// with the queue set aside and puts it back afterwards.
#include <algorithm>
#include <utility>
#include <vector>
#include "ocgapi.h"
#include "duel.h"
#include "interpreter.h"
#include "field.h"
#include "card.h"
#include "effect.h"

namespace {
template<typename Query>
void* aside(OCG_Duel ocg_duel, Query query) {
	auto& params = static_cast<duel*>(ocg_duel)->lua->params;
	auto queued = std::move(params);
	params.clear();
	void* result = query();
	params = std::move(queued);
	return result;
}
}

extern "C" void* YO3_DuelQuery(OCG_Duel ocg_duel, uint32_t* length, const OCG_QueryInfo* info) {
	return aside(ocg_duel, [&] { return OCG_DuelQuery(ocg_duel, length, info); });
}

extern "C" void* YO3_DuelQueryLocation(OCG_Duel ocg_duel, uint32_t* length, const OCG_QueryInfo* info) {
	return aside(ocg_duel, [&] { return OCG_DuelQueryLocation(ocg_duel, length, info); });
}

extern "C" void* YO3_DuelQueryField(OCG_Duel ocg_duel, uint32_t* length) {
	return aside(ocg_duel, [&] { return OCG_DuelQueryField(ocg_duel, length); });
}

// What lasting effects of the field's cards apply to each face-up monster, for the client's relation markers (the
// server's `sync_relations`, docs/protocol.md "Card relations"): 6 bytes each, {source controller, source location,
// source sequence, monster controller, monster sequence, kind}. Kind 3: a Field Spell's effect; 7: a negation (a
// continuous card's or a monster's, Debris Dragon's on the monster it brings back); 5: another continuous Spell or
// Trap's (Gravity Bind, Messenger of Peace, Level Limit - Area B). Equips and targets are left out: their own messages
// say them. A monster's effects on monsters (Command Knight) are left out too, as Konami's server did (its logs).
// Checking an effect can call Lua (its condition, its target function), so it runs with the queue set aside.
namespace {
constexpr uint8_t RELATION_FIELD = 3;
constexpr uint8_t RELATION_CONTINUOUS = 5;
constexpr uint8_t RELATION_NEGATED = 7;

bool negation(uint32_t code) {
	return code == EFFECT_DISABLE || code == EFFECT_DISABLE_EFFECT;
}

void add(std::vector<uint8_t>& out, card* source, card* monster, uint8_t kind) {
	uint8_t entry[6] = { source->current.controler, source->current.location, static_cast<uint8_t>(source->current.sequence),
		monster->current.controler, static_cast<uint8_t>(monster->current.sequence), kind };
	for(size_t i = 0; i + 6 <= out.size(); i += 6)
		if(std::equal(entry, entry + 6, out.begin() + i))
			return;
	out.insert(out.end(), entry, entry + 6);
}

std::vector<uint8_t> relations;
}

extern "C" void* YO3_DuelRelations(OCG_Duel ocg_duel, uint32_t* length) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	return aside(ocg_duel, [&]() -> void* {
		relations.clear();
		auto& players = pduel->game_field->player;
		std::vector<card*> sources;
		for(auto& p : players) {
			for(auto* c : p.list_mzone)
				if(c && c->is_position(POS_FACEUP))
					sources.push_back(c);
			for(auto* c : p.list_szone)
				if(c && c->is_position(POS_FACEUP))
					sources.push_back(c);
		}
		for(auto& p : players) {
			for(auto* monster : p.list_mzone) {
				if(!monster || !monster->is_position(POS_FACEUP) || monster->current.sequence >= 5)
					continue;
				for(auto* source : sources) {
					if(source == monster)
						continue;
					bool spell_trap = source->current.location == LOCATION_SZONE;
					bool field_spell = spell_trap && source->current.sequence == 5;
					for(auto& [code, e] : source->field_effect) {
						if(!(e->type & EFFECT_TYPE_FIELD) || e->is_flag(EFFECT_FLAG_PLAYER_TARGET))
							continue;
						if(!negation(code) && !spell_trap)
							continue;
						if(!e->is_available() || !e->is_target(monster))
							continue;
						add(relations, source, monster, negation(code) ? RELATION_NEGATED : field_spell ? RELATION_FIELD : RELATION_CONTINUOUS);
					}
				}
				// A negation put on the monster itself by a card still on the field.
				for(auto& [code, e] : monster->single_effect) {
					card* owner = e->get_owner();
					if(!negation(code) || !owner || owner == monster || !(owner->current.location & LOCATION_ONFIELD) || !owner->is_position(POS_FACEUP))
						continue;
					if(e->is_available())
						add(relations, owner, monster, RELATION_NEGATED);
				}
			}
		}
		*length = static_cast<uint32_t>(relations.size());
		return relations.empty() ? nullptr : relations.data();
	});
}
