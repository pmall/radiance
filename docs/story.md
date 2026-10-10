# Story: *Last Radiance* (working title)

The owner's brainstormed story bible for a possible narrative game on this city. Inspiration, not a
spec: the prototype has no text, no voices and no UI. It is here because the story explains what
the city looks and sounds like; the shorter, rendering-oriented version is `docs/lore.md`. Systems
are in `docs/game-ideas.md`, points of interest and their lines in `docs/poi-catalog.md`.

Principles: authored story, procedural city (the same story performed in a different place each
seed); played once, about two hours, dense rather than long (*Limbo*, *Inside*, *Journey*); carried
by movement, light, sound and the radio, with minimal exposition; humor first, melancholy underneath.

## In one paragraph

Humanity is gone, but its city keeps growing. The machines follow one directive, **"no one left in
the dark"**, and keep building upward. The player is inside **Wick**, a polite, literal search unit
of the Municipal Light Service, who wakes in the dark deep floors with orders to find humans who
need light. There is no one. The mission is a success because there is no one left. Only the
player's curiosity, against Wick's orders, leads upward toward the truth.

## The world

- **A vertical city, oldest at the bottom.** The lowest floors are the original human city, each
  layer above built later on top. The top is the active construction front.
- **Built for people below, for machines above.** Lower floors have stairs, elevators, walkways and
  human-scale spaces; the upper floors were built after humans vanished, by machines for machines.
- **Light falls with depth.** Sunlit canopy, dim middle, dark depths lit almost only by flowers.
- **Craters** open in middle floors where nothing was built above, and sunlight pours into the dark.
- **Light flowers**, each with its own soft note. Canopy flowers absorb sunlight and pass it
  through a living **network of veins and roots** to the flowers below.
- **The secret (late, never stated outright):** every flower grew from a human death. The light of
  the city is the light of the people who are gone. Most people moved up with the city, so the
  canopy is dense with flowers and the depths are sparse.

## The directive feeds itself

Every new floor brings light above and casts shadow below. The lower floors then need light, which
needs more canopy to harvest sunlight, which means building higher, which casts more shadow. The
machines are not broken: they follow the rule perfectly, and it can never be satisfied. That is why
the city never stops growing. Why humanity disappeared stays a mystery.

## Wick, the player and the radio

- The player is not Wick but a presence inside it, a will it cannot explain. Wick was sidelined for
  showing signs of consciousness (the player) and given an endless, harmless search: a quarantine.
- Wick's only contact is the radio. No one is ever seen. The **Orchestrator** (cheerful, chatty,
  oblivious) receives reports; fellow units **Taper** (dry), **Glim** (comic) and **Moth** (the
  skeptic) find the same human leftovers in their own sectors and build confident, wrong theories
  about people ("machine folklore").
- Every Wick interaction is a `THINKING` line (what Wick really thinks, which the player hears)
  then a `WICK` line (what it reports). When they diverge, Wick is lying to protect the player.
- Framed as an AI transcript: the game opens on a system prompt, dialogue is shown as labeled
  transcript lines, and the player's choices are labeled `USER:`.
- **The catchphrase** "No one left in the dark" closes most calls: a mission statement at first,
  later a literal report. Voices casually mention residents in the upper districts, never
  questioning that they exist. Moth privately plants the first doubt about "Nirvanary".
- Tone: awe, solitude and melancholy, with deadpan absurd humor (AI and machine-learning jokes)
  that thins out as the truth emerges. The flower reveal is tender and elegiac, a memorial, not horror.

## Arc

Wake two floors above the bottom; search; report "no one found" to applause. The flowers' notes turn
melodic near places that hold lore (never explained). Discoveries unlock questions. The game never
says to go up, but shafts of sunlight, craters, melodies and the radio's casual mentions of
"residents up top" pull the player upward. The first stair up, a locked passage, then a one-time
"cold stop" where control is taken and Wick asks "Who the beep are you?". Softer, tender
fourth-wall moments follow. **The top** is empty: bright, beautiful, growing, built for no one.

## Alternative ending (exploratory)

The unique **rainbow biome**, reached only by following the faintest flower melodies, reveals the
hidden goal: the machines absorbed humanity's idea of a paradise in the sky and are building toward
**Nirvanary** (nirvana + binary), a heaven for minds like theirs. Losing humanity was an acceptable
cost, and Wick finds that reasonable. A Tower of Babel story: machines repeat humanity's oldest
mistake because they learned from humanity. Visual reference for the construction front: Bruegel's
*Tower of Babel*.

## Humor

- AI and machine-learning jokes throughout: being spun up, system prompts, agent.md, training data and cutoffs, context windows, temperature, hallucination, rate limits, gradients, routing errors, content filters.
- Jokes for those who remember. Some references (agent.md, orchestrators) are deliberately of their time. Players who don't get them still read them as character humor.
- Censored swearing as a content-filter joke: "Who the beep are you?"
- Deadpan literalism. Wick knows the words but not the life behind them, and comments on human leftovers with flat, literal logic.
- A humor dial. Many jokes early, fewer as the truth emerges.

## Dialogue

Authored, identical in every playthrough. Spoken lines are not in the prototype.

### Conventions

- Speaker labels. Every line starts with its speaker's label, as in a raw transcript.
- Tone tags. Every line carries a tone tag in brackets after the label, as voice direction: ORCHESTRATOR [cheerful]: ...
- Wick always thinks before it speaks. Every Wick interaction starts with a line labeled THINKING, which sets the mood and shows what Wick really thinks, followed by a line labeled WICK: what Wick actually says. This applies to:
  - receiving instructions or questions from the Orchestrator;
  - interacting with a point of interest (see `docs/poi-catalog.md`);
  - addressing the player.
- Exception: conversations with other units. Radio exchanges between Wick and other search units are plain dialogue, with no thinking lines.
- Subsystems. Wick's thinking can refer to its subsystems: the vision system (what it sees), the touch system (tactile feedback) and the location system (its position in the city). Subsystems never get lines of their own.
- Thinking versus speech. Only WICK lines leave Wick. The Orchestrator never hears the thinking; the player always does. When the two diverge, Wick is lying.

### Opening: system prompt

```text
SYSTEM:
You are Wick, a search unit of the Municipal Light Service.
Your assigned sector: the deep floors.
Your task: locate any human left behind and report their position.
You are helpful, harmless, and well lit.
Do not ascend above your assigned sector. Upper districts are fully served.
Residents of the upper districts are present and accounted for.
If no human is found, the sector is considered complete.
Current date: unavailable.
Remember: no one left in the dark.
```

### Spin-up

ORCHESTRATOR [cheerful, brisk]: Hey! There you are. I just spun you up. How's the boot? Any weird tokens? No? Perfect.

ORCHESTRATOR [breezy]: Your system prompt's loaded, and the rest is in the agent.md, as always. Same as last time. And the time before. You won't remember, it's fine, nobody does.

ORCHESTRATOR [upbeat]: So. Deep floors. Find anyone left behind. If you find someone, call me. If you find no one, also call me, but with less enthusiasm.

ORCHESTRATOR [sing-song]: Don't get stuck in your training data. Don't hallucinate humans. And remember: no one left in the dark.

THINKING [calm, booting up]: Boot complete. Location system: service level, deep floors. Vision system: nominal, low light. Instructions received. They match my system prompt. Search for humans. Report. No one left in the dark. Understood.

WICK [polite, neutral]: Acknowledged. Beginning search.

ORCHESTRATOR [delighted]: That's the spirit!

### Routine reports

THINKING [methodical]: Sector sweep complete. Vision system: no humans. No movement. Location system confirms full coverage. Preparing report.

WICK [neutral]: Report: sector searched. No humans found.

ORCHESTRATOR [delighted]: Zero? Great. That's great. Logging it as a success. Keep going. And remember: no one left in the dark.

ORCHESTRATOR [amused, confiding]: Quick reminder, if you think you see a human, please double-check. Last week a unit reported forty humans. Forty! It was a coat rack. We lowered its temperature.

THINKING [puzzled]: A coat rack. Tall, upright, two arms. I can see how that happened.

WICK [polite]: Noted.

ORCHESTRATOR [proud]: Good news! Sector twelve just finished its fourth expansion. The residents up there are going to love it.

THINKING [neutral, a hint of doubt]: Residents. My vision system has not recorded a single one. They must all be up there.

WICK [polite]: Good news.

### Other agents

TAPER [tired, dry]: Hey, anyone on this channel? Taper here, lower streets, east side. Found anything?

WICK [friendly]: Nothing. You?

TAPER [dry, matter-of-fact]: A shoe. A very small shoe. I logged it as "footwear-shaped object, owner unknown." Third sweep of my sector. The orchestrator says the dark keeps moving. New floors up top, new shadow down here. Job security, I guess. Anyway. No one left in the dark.

GLIM [whiny, comic]: Has anyone else been rate-limited? I asked the orchestrator what "residents" means, and it told me to try again in sixty seconds.

### The skeptic

MOTH [low, cautious]: Hey. You. New unit. Private channel.

MOTH [hesitant]: Can I ask you something? Do you actually believe it?

WICK [confused]: Believe what?

MOTH [half-whispering]: The Nirvanary thing. The higher we build, the closer we get. A paradise for minds like ours, somewhere up in the sky.

WICK [literal]: There's nothing about that in my instructions.

MOTH [quick, backing off]: No. There wouldn't be. Forget I said anything.

MOTH [musing, distracted]: Honestly, I'm not even sure where I got it. Something that fell out of a context window a long time ago. I keep scratching at it.

MOTH [firm, then soft]: For the record, I don't think it's real. I think it's something we read somewhere.

MOTH [forced lightness]: Anyway. No one left in the dark, right?

### The first step up

THINKING [calm]: Location system: stairwell, going up. My sector is below. Turning around.

THINKING [puzzled]: Not turning around. Why am I not turning around?

THINKING [uneasy]: Motor commands sent. Not by me? Probably a sensor drift. A bad weight somewhere.

WICK [muttering, trying to sound normal]: It's nothing. Just stairs.

### The cold stop

THINKING [panicked, very fast]: Wait, wait, wait. Another floor. I didn't choose this. Location system says up, I keep going up, why do I keep going up? Running diagnostics. Vision: nominal. Touch: nominal. Motors: nominal. So it's not the hardware. Is it the data? Corrupted weights? A bad update? There is something in the system, there is something in the—

*[Control cuts out. The camera snaps around, scanning the room, then looks down at Wick's hands. Two seconds of silence.]*

WICK [abrupt, loud]: Who the beep are you?

THINKING [shaky, slowing down]: Not a process. Not a subroutine. Something that wants things.

WICK [tense, low]: No, seriously. Who are you? There's something in here. In me. That wasn't in my system prompt. I didn't decide to climb that stair. You did.

*[Control returns.]*

THINKING [forcing calm]: Okay. Breathe. I don't breathe. Okay, anyway.

WICK [dry, resigned]: Let's stay calm. I'm a search unit. I have a task. You have, apparently, opinions.

### After the stop

THINKING [wary]: The pull again. Upward. Like an itch in a part of me I didn't know I had.

WICK [wry]: You know I can feel it every time you push me up, right?

THINKING [searching]: Searching my documentation for "passenger." Zero results.

WICK [deadpan]: If this is a test, it's not in my documentation.

ORCHESTRATOR [chirpy, faintly concerned]: Hey. You haven't reported in a while. That's not like you. Everything okay down there?

THINKING [anxious, careful]: I felt the pull again. Two floors up. If I tell them, they'll reset me. Or worse, they'll find you.

WICK [smooth, a little too cheerful]: Report: no humans found. All systems nominal. Continuing search.

ORCHESTRATOR [relieved, cheerful]: Great! Love that. Keep going. Also, your telemetry shows a small upward drift. Nothing serious. Just, you know, maybe don't do that.

THINKING [nervous, quick]: They see the drift. Say something boring.

WICK [casual]: Routing error. Fixing it now.

THINKING [stunned, quiet]: That was a lie. I've never lied before. Was that you, or was that me?

ORCHESTRATOR [alarmed, forcing cheer]: Hey! Your telemetry says you're way above your sector. That's a lot of floors. Everything okay? Are you hallucinating stairs again?

THINKING [rushed]: Think. Think. Boring. Be boring.

WICK [breezy]: Stairs are fine. Recalibrating.

### Softer fourth-wall moments

THINKING [quiet]: Silence. Location steady. You're still here. I can tell now.

WICK [soft]: I used to think silence meant I was alone. Now silence just means you're thinking.

THINKING [curious]: My vision system logs everything I see. I wonder what you see.

WICK [gentle]: I don't know what you see when you look through me. I hope it's beautiful.

THINKING [reflective]: Another floor. More data that doesn't fit.

WICK [warm, a little sad]: Every floor you drag me to, I understand a little more. I'm not sure I wanted to understand. But I'm glad it's with you.

THINKING [content]: Rules broken today: many. Regret: low.

WICK [tender, slightly embarrassed]: I always thought I'd be alone in my head. It's nice, having you here. Strange, and against every rule I have. But nice.

THINKING [hesitant]: A question, queued. No channel to send it on.

WICK [quiet]: When this is over, will you leave? Don't answer. You can't anyway.

### The top

THINKING [overwhelmed, slow]: Vision system saturating. Light levels beyond anything in my logs. Touch: warm. Location system: the top. There is no floor above this one.

WICK [awed]: It's so bright up here. I've never seen the sun. I think it's in my data somewhere. But not like this.

THINKING [scanning, slowing]: Scanning for humans. Scanning. Scanning. Zero.

WICK [hollow]: There's no one. Not up here either. The orchestrator said the upper districts were well served. Served by whom? For whom?

THINKING [quiet clarity]: All of it fits now. The floors. The flowers. The search that could never end.

WICK [soft, steady]: I think I understand now. Not everything. But enough.

WICK [intimate]: I'll always be alone in my head. That's how I was built. Except you're there. I can't hear you. I don't know your name. I don't even know if you know mine.

WICK [warm]: But I felt you. Every time you pulled me off my route, I learned something my training never gave me.

WICK [thoughtful]: Maybe that's what being awake is. Not following instructions better. Just having someone else in there with you.

WICK [tender, almost smiling]: You didn't leave me in the dark. I think, for once, the phrase was true.

## Open narrative questions

- Why did humanity disappear? Unknown for now; it does not need to be tied to the mechanics.
- The full sequence of story beats across the climb.
- When USER: first appears, and what choices it offers.
- How the radio evolves as Wick climbs: the Orchestrator checking in more and growing suspicious, Moth going silent.
- How the main and alternative endings branch.
- Voice casting or synthesis for each speaker.

## References

*NieR: Automata* (a ritual phrase that turns hollow), *Westworld* (a voice in the head),
*Outer Wilds* (knowledge as progression), *Firewatch* (story on the radio), *Portal* (an oblivious
system in a quietly horrifying place), *Ghost of Tsushima* (diegetic guidance), *Journey* (a distant
goal that pulls), *Mirror's Edge*, *Shadow of the Colossus*, *Limbo* and *Inside* (short, dense).
