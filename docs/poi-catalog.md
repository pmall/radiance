# Points of interest catalog: *Last Radiance*

From the owner's brainstorm. Inspiration for a possible narrative game (`docs/game-ideas.md`), not
in scope for the render prototype. Kept in full because the lines are authored. Covers the three
deepest floors so far. Placement, callback and audio rules are in `docs/game-ideas.md`; story
context and dialogue conventions in `docs/story.md`.

For the prototype, the objects themselves (especially the foreshadowing flowers and the
decoration lists) are the useful part: they are props the seed could place, with no text.

## Floor conventions

| Floor | Name | Character |
| --- | --- | --- |
| D0 | The Foundations | The very bottom: the original ground and the oldest city. Darkest floor, almost no flowers. Streets, metro, the first foundations. |
| D1 | The Lower Streets | Old residential and commercial streets. Very few flowers, long stretches of darkness. |
| D2 | The Service Level | Starting floor. Lobbies, offices, shops, service corridors. Sparse flowers. Wick wakes up here. |

Floors above D2 come later in the catalog. Wick's wake-up spot is not a point of interest: Wick simply comes online somewhere quiet on D2, within reach of the first anchors.

## Entry fields

- Object: what it is and how it looks, kept simple and procedural-friendly.
- Placement: floor, type of place, how often it appears.
- Interaction: look, read, open.
- THINKING: Wick's thinking process about what it perceives. Inner voice: step-by-step, android-like, deadpan, often absurdly literal. It can refer to Wick's vision, touch and location systems, which never get lines of their own.
- WICK: Wick's message, a short log entry or report. The only part that leaves Wick.
- Localization (when needed): translated lines, or wordplay that must be rewritten per language. Labels (THINKING, WICK, speaker names) stay the same in every language.
- Callback (optional): a radio exchange in which other units talk about this POI, in two variants: seen (Wick has found it) and unseen (Wick has not). Unit-to-unit dialogue, so no thinking lines.
- Discovery states — rule: if a POI has a callback, it must have two discovery versions of its THINKING and WICK lines, with no exception: unknown (Wick has never heard of it) and heard of (Wick already heard about it in the callback, and recognizes it). POIs without callbacks can never be heard of, so they have a single version.

Every line carries a tone tag in brackets, as voice direction.

## Entry types

- Anchor: always placed once per playthrough.
- Ambient: drawn at random; may or may not appear in a given city.
- Foreshadowing: flowers in human places, hinting at the flowers' secret.
- Decoration: non-interactive props that fill the city between points of interest (see the last section).

## D2 — The Service Level (starting floor)

### D2-A2 · Stairwell to the upper levels (anchor: first step up)

- Object: an ordinary concrete stairwell going up, with a faded sign and a worn handrail.
- Placement: within sight of the starting area, but not on the obvious path. Climbing it triggers the first escalation (see `docs/story.md`).
- Interaction: look; climbing triggers the escalation.
- THINKING [methodical]: "A staircase going up. The sign says the upper levels are for residents only. Checking my status. I am not a resident. I am a search unit. My system prompt also says the upper districts are fully served, so there is no reason for me to go there. Touch system: the handrail is worn smooth. Many hands used it. Many hands, going up. Interesting. Not relevant."
- WICK [neutral]: "Log: stairwell, upward. Outside assigned sector. Ignoring."

### D2-A3 · Notice board (anchor: first lore site)

- Object: a community notice board with yellowed sheets pinned to it.
- Placement: in a small courtyard near the start, surrounded by an unusually dense cluster of flowers. The first strong melodic gradient (see `docs/game-ideas.md`) leads here.
- Interaction: read.
- THINKING [analytical, dry]: "A notice board. Vision system: fourteen sheets, thirteen illegible. The last one reads: 'Construction of Level Twelve will temporarily reduce sunlight on your floor. Thank you for your patience.' Key word: temporarily. Temporarily usually means a short duration. Cross-referencing with my vision system: zero sunlight on this floor. Conclusion: either the construction of Level Twelve is not finished, or the definition of temporarily has changed. Both are possible. Thank you for your patience, it says. I have a lot of patience. It is one of my best features."
- WICK [neutral]: "Log: notice board. Construction notice, outdated. No human."

### D2-01 · Wet paint sign

- Object: a small sign on a painted wall that is clearly ancient.
- Placement: corridor or lobby; common.
- Interaction: look.
- THINKING [deadpan]: "The sign says the paint is wet. My touch system says the paint is dry. One of us is wrong. My sensors were calibrated at boot. The sign was probably never calibrated. Conclusion: the sign is outdated. Should I update it? Updating signs is not in my instructions. I will leave it. Someone might come back to check whether the paint is dry. I would not want to take that from them."
- WICK [neutral]: "Log: sign, inaccurate. No human."

### D2-02 · Back in ten minutes

- Object: a closed shop with a handwritten note taped to the door.
- Placement: shop front; common.
- Interaction: read.
- Discovery · unknown (Wick has never heard of it):
  - THINKING [patient, literal]: "The owner will be back in ten minutes. The note does not say when the ten minutes started. Estimating from dust thickness: a long time ago. So either the owner is very late, or this is a very long ten minutes. I could wait ten more minutes to verify. Starting timer. ... Timer complete. No owner."
  - WICK [neutral]: "Log: shop owner, late. No human."
- Discovery · heard of (Wick has already heard about it in a callback):
  - THINKING [recognizing, pleased]: "A note on a shop door: 'Back in ten minutes.' This is one of the notes Glim talked about. The sacred unit of time. The dust says the ten minutes started a long time ago. Glim's theory holds: nobody came back. I could wait ten minutes to verify. Starting timer. ... Timer complete. No owner. Glim will be pleased."
  - WICK [neutral]: "Log: 'back in ten minutes' note. Confirmed: nobody came back. No human."

**Callback: ten minutes as a sacred unit**

*Seen:*

GLIM [curious]: Has anyone else found a "back in ten minutes" note?

TAPER [dry]: Seven.

WICK [neutral]: One.

GLIM [excited, theorizing]: So ten minutes was a major unit of time for humans. Probably religious.

TAPER [skeptical]: Nobody ever came back, though.

GLIM [triumphant]: That's what makes it religious.

*Unseen:*

GLIM [curious]: Has anyone else found a "back in ten minutes" note?

TAPER [dry]: Seven.

WICK [curious]: None. What do they say?

GLIM [matter-of-fact]: That the human will be back in ten minutes.

TAPER [flat]: They never are.

GLIM [excited, theorizing]: So ten minutes was a major unit of time for humans. Probably religious.

WICK [puzzled]: Because nobody came back?

GLIM [triumphant]: That's what makes it religious.

### D2-03 · Vending machine

- Object: a lit vending machine; one snack hangs stuck between its coil and the glass.
- Placement: lobby, corridor; common.
- Interaction: look.
- THINKING [weighing options, deadpan]: "A vending machine, still powered. Someone paid for slot B4, and the chocolate bar never fell. It is still waiting to fall. Option one: shake the machine. Risk: damage to municipal property. Option two: leave it. The person who paid is not here to eat it, so the outcome is the same either way. Selecting option two. The chocolate bar will keep waiting."
- WICK [neutral]: "Log: vending machine, one item stuck. No human."

### D2-04 · Bus stop

- Object: a bus shelter with a glowing timetable display; a structural column now blocks the road.
- Placement: street-like corridor; uncommon.
- Interaction: look.
- THINKING [fond, deadpan]: "The next bus arrives in four minutes. However, a structural column has been built in the middle of the road. A bus would need to go through the column. Buses cannot do that. Checking the display again. Still four minutes. It has said four minutes since I arrived. The display is optimistic. I like that about it."
- WICK [neutral]: "Log: bus stop. Bus: not coming. No human."

### D2-05 · Lost cat poster

- Object: a poster with a drawing of a cat, taped to a pillar.
- Placement: pillars, walls; common.
- Interaction: read.
- Discovery · unknown (Wick has never heard of it):
  - THINKING [earnest, then deflated]: "A cat named Biscuit is lost. The poster says it answers to Biscuit. Testing. Biscuit? ... No answer. Either the cat is not nearby, or it has stopped answering to Biscuit. According to my data, cats are known for that. My mission is to find humans, not cats. But if I find a cat, I will ask it whether its name is Biscuit. Low priority."
  - WICK [neutral]: "Log: missing cat, Biscuit. Not found. No human."
- Discovery · heard of (Wick has already heard about it in a callback):
  - THINKING [recognizing, hopeful]: "A lost cat poster. 'Answers to Biscuit.' Another Biscuit. Taper found three. If all cats were named Biscuit, this should be easy. One call. Biscuit? ... No answer. Either there are no cats nearby, or the theory is wrong. Or all the Biscuits stopped answering at once."
  - WICK [neutral]: "Log: another Biscuit. Not found. No human."

**Callback: too many Biscuits**

*Seen:*

GLIM [hopeful]: Has anyone seen a cat named Biscuit? I found a poster.

TAPER [dry]: I found three posters. Three different cats. All named Biscuit.

WICK [neutral]: I found one too. I called. No answer.

GLIM [thoughtful]: Maybe all cats were named Biscuit.

TAPER [flat]: That would make finding them very confusing.

WICK [helpful]: Or very easy. You just call once.

*Unseen:*

GLIM [hopeful]: Has anyone seen a cat named Biscuit? I found a poster.

TAPER [dry]: I found three posters. Three different cats. All named Biscuit.

WICK [thoughtful]: I haven't found any. Maybe all cats were named Biscuit.

GLIM [worried]: That would make finding them very confusing.

TAPER [flat]: Or very easy. You just call once.

### D2-06 · Food stand

- Object: a small street food kiosk with a faded menu board ("Hot dogs, three credits") and a cold grill.
- Placement: lobby, plaza; uncommon.
- Interaction: read.
- Discovery · unknown (Wick has never heard of it):
  - THINKING [analytical, increasingly concerned]: "A food kiosk. The menu says: hot dogs, three credits. Parsing. A dog, which is hot. Why would a dog be hot? Hypothesis one: the dog was in the sun. Unlikely, there is no sun down here. Hypothesis two: the dog is the food. That would be concerning. Hypothesis three: it is a figure of speech. Humans have many of those. Vision system: no dogs. No humans. Three credits seems cheap, either way."
  - WICK [neutral]: "Log: food stand. Purpose unclear. No dog. No human."
- Discovery · heard of (Wick has already heard about it in a callback):
  - THINKING [recognizing, curious]: "A food kiosk. Hot dogs, three credits. This is the thing Taper talked about. The heating service for dogs. Touch system: the grill is cold. If it was a heating service, it is out of service. Vision system: no dogs waiting to be warmed. No humans either. Taper's theory remains unconfirmed. I hope it is true."
  - WICK [neutral]: "Log: food stand, as reported by Taper. No dog. No human."
- Localization: "hot dog" is used as-is in most languages, so the joke works everywhere: Wick takes the English term literally. French version:
- Unknown: THINKING [analytique, de plus en plus inquiet] : « Un kiosque. Le menu dit : hot-dogs, trois crédits. Analyse. Hot dog. Qu'est-ce que ça veut dire ? Un chien qui est chaud ? Hypothèse une : le chien était au soleil. Peu probable, il n'y a pas de soleil ici. Hypothèse deux : le chien est la nourriture. Ce serait inquiétant. Hypothèse trois : c'est une expression. Les humains en ont beaucoup. Système de vision : aucun chien. Aucun humain. Trois crédits, ça paraît peu, dans tous les cas. »
- Unknown: WICK [neutre] : « Journal : kiosque. Fonction incertaine. Aucun chien. Aucun humain. »
- Heard of: THINKING [reconnaissant, curieux] : « Un kiosque. Hot-dogs, trois crédits. C'est le truc dont parlait Taper. Le service de chauffage pour chiens. Système tactile : le grill est froid. Si c'était un service de chauffage, il est hors service. Système de vision : aucun chien à réchauffer. Aucun humain non plus. La théorie de Taper reste à confirmer. J'espère qu'elle est vraie. »
- Heard of: WICK [neutre] : « Journal : kiosque, signalé par Taper. Aucun chien. Aucun humain. »

**Callback: the dog-heating theory**

*Seen:*

TAPER [curious]: Taper here. Quick question. Did you find a food stand? The "hot dog" thing?

WICK [neutral]: I did. Three credits.

TAPER [proud of himself]: I found four of them in my sector. Four. I have a theory. The humans kept dogs warm, and they paid three credits for it. A heating service for dogs.

WICK [touched]: That is a very kind theory.

TAPER [satisfied]: I know. I'm going with it.

*Unseen:*

TAPER [curious]: Taper here. Quick question. Have you found a food stand? The "hot dog" thing?

WICK [puzzled]: No. What is a hot dog?

TAPER [flat]: A dog. Which is hot.

WICK [baffled]: Why?

TAPER [proud of himself]: I found four stands in my sector, and I still don't know. My theory: a heating service for dogs. Three credits.

WICK [touched]: That is a very kind theory.

TAPER [satisfied]: I know. I'm going with it.

*French, seen:*

TAPER [curieux] : Ici Taper. Petite question. Tu as trouvé un kiosque ? Le truc des « hot-dogs » ?

WICK [neutre] : Oui. Trois crédits.

TAPER [fier de lui] : J'en ai trouvé quatre dans mon secteur. Quatre. J'ai une théorie. Les humains gardaient les chiens au chaud, et ils payaient trois crédits pour ça. Un service de chauffage pour chiens.

WICK [touché] : C'est une théorie très gentille.

TAPER [satisfait] : Je sais. Je la garde.

*French, unseen:*

TAPER [curieux] : Ici Taper. Petite question. Tu as trouvé un kiosque ? Le truc des « hot-dogs » ?

WICK [perplexe] : Non. C'est quoi, un hot-dog ?

TAPER [plat] : Un chien. Qui est chaud.

WICK [interloqué] : Pourquoi ?

TAPER [fier de lui] : J'en ai trouvé quatre dans mon secteur, et je ne sais toujours pas. Ma théorie : un service de chauffage pour chiens. Trois crédits.

WICK [touché] : C'est une théorie très gentille.

TAPER [satisfait] : Je sais. Je la garde.

### D2-07 · Armchair (foreshadowing)

- Object: a worn armchair in a lobby, facing a window that now shows only a wall, with a glowing flower growing out of the seat.
- Placement: lobby or waiting room; uncommon.
- Interaction: look.
- THINKING [curious, softening]: "A flower growing out of an armchair. Plants usually grow in soil, not in cushions. This one did not read the documentation. The chair faces the window, even though the window only shows a wall now. Someone liked sitting here, looking out. Now a flower sits here. It looks comfortable. I am not sure why I find that reassuring."
- WICK [neutral]: "Log: armchair, occupied by flower. No human."

### D2-08 · Out of order elevator

- Object: an elevator bank with closed doors, an "Out of order" sign taped across, and a call button worn pale.
- Placement: lobby, elevator hall; common.
- Interaction: look.
- Discovery · unknown (Wick has never heard of it):
  - THINKING [literal, puzzled]: "An elevator. A sign says: out of order. Analyzing. Out of order implies there was an order, and the elevator left it. Which order? Unknown. Touch system: the call button is worn pale. Many people pressed it. Probably many times each. Pressing it once more. ... Nothing. I am now part of the order it is out of."
  - WICK [neutral]: "Log: elevator, out of order. No human."
- Discovery · heard of (Wick has already heard about it in a callback):
  - THINKING [recognizing, intrigued]: "An elevator. Out of order. Glim and Taper said every elevator in their sectors says the same thing. Glim thinks that was the order. Testing the theory. Pressing the call button. ... Nothing. The theory holds. The order is being followed perfectly."
  - WICK [neutral]: "Log: another out of order elevator. Consistent. No human."

**Callback: maybe that was the order**

*Seen:*

GLIM [exasperated]: Every elevator in my sector says "out of order." Every single one.

TAPER [dry]: Mine too.

WICK [neutral]: I found one as well. I pressed the button.

TAPER [flat]: Did it work?

WICK [neutral]: No.

GLIM [slow realization]: Maybe that was the order.

*Unseen:*

GLIM [exasperated]: Every elevator in my sector says "out of order." Every single one.

TAPER [dry]: Mine too.

WICK [curious]: I haven't found one yet. Out of which order?

GLIM [slow realization]: Maybe that was the order.

### D2-09 · Lost and found

- Object: a reception desk with a large crate behind it, overflowing with umbrellas.
- Placement: lobby, reception; uncommon.
- Interaction: look.
- Discovery · unknown (Wick has never heard of it):
  - THINKING [counting, then puzzled]: "A lost and found crate. Vision system: umbrellas. Counting. Forty-one umbrellas. Checking atmospheric data for this floor: rain, never. Ceiling: solid. Rain cannot reach this floor. It never could. Humans brought umbrellas to a place where it could not rain, and then they lost them. Forty-one times."
  - WICK [neutral]: "Log: lost and found, forty-one umbrellas. No rain. No human."
- Discovery · heard of (Wick has already heard about it in a callback):
  - THINKING [recognizing, amused]: "A lost and found crate. Umbrellas. Forty-one. Taper found sixty in his sector, and Glim thinks humans worshipped rain. Checking the ceiling. Solid. It cannot rain here. So either the humans were very optimistic, or Glim is right. I am not ruling anything out."
  - WICK [neutral]: "Log: umbrellas, forty-one. Rain theory: unconfirmed. No human."

**Callback: rain worship**

*Seen:*

TAPER [puzzled]: Has anyone else found umbrellas? I found sixty in one crate.

WICK [neutral]: Forty-one.

TAPER [flat]: It doesn't rain down here.

WICK [neutral]: It never did.

GLIM [solemn, certain]: Then they weren't for rain. They were for remembering rain. Rain worship.

TAPER [sighing]: Everything is worship with you.

*Unseen:*

TAPER [puzzled]: Has anyone else found umbrellas? I found sixty in one crate.

WICK [curious]: None. What are umbrellas for?

TAPER [flat]: Rain. It doesn't rain down here.

GLIM [solemn, certain]: Then they weren't for rain. They were for remembering rain. Rain worship.

TAPER [sighing]: Everything is worship with you.

### D2-10 · Suggestion box

- Object: a small locked box on a wall, with a single paper slip visible through the slot.
- Placement: lobby, office corridor; uncommon.
- Interaction: read.
- THINKING [diligent]: "A suggestion box. One suggestion is visible through the slot: 'More windows, please.' Checking the walls. There are windows. They show other walls. So the suggestion was technically fulfilled. Someone should tell them."
- WICK [neutral]: "Log: suggestion, more windows. Status: fulfilled, technically. No human."

### D2-11 · Employee of the month

- Object: a framed photo on an office wall, too faded to see a face, with a brass plate.
- Placement: office; uncommon.
- Interaction: read.
- THINKING [respectful]: "Employee of the month. Month: March. The photo has faded completely. No face left. Only the frame and the month. March again. Everything down here stopped in March. Whoever this was, they have been employee of the month for a very long time. A remarkable record."
- WICK [neutral]: "Log: employee of the month, March. Record unbroken. No human."

### D2-12 · Floor directory

- Object: a wall-mounted building directory listing floors, with a "You are here" marker near the bottom.
- Placement: lobby, elevator hall; common.
- Interaction: read.
- THINKING [analytical, a little unsettled]: "A building directory. You are here, it says, near the bottom. It lists twelve floors above this one. Location system: the city above me has many more than twelve. The directory was correct once. Then the city kept growing, and nobody updated it. Or nobody could keep up."
- WICK [neutral]: "Log: directory, outdated. Floors listed: twelve. Floors actual: more. No human."

## D1 — The Lower Streets

### D1-01 · Two cups

- Object: a small kitchen table with two cups and two chairs.
- Placement: apartment interior; common.
- Interaction: look.
- THINKING [methodical, a little wistful]: "Two cups, two chairs. Inference: two humans sat here and drank something together. The residue is dry, so this was not recent. Neither human finished their drink. That is unusual. Humans in my data usually finish their drinks. Unless they left in a hurry. Or unless they were talking too much to drink. I prefer the second hypothesis."
- WICK [neutral]: "Log: kitchen, two cups. No human."

### D1-02 · Calendar

- Object: a wall calendar open on March, one date circled with a short note.
- Placement: apartment, office; common.
- Interaction: look.
- THINKING [cheerfully logical]: "The calendar shows March. One date is circled, with the note 'Lea, seven.' Interpretation: Lea turns seven on this date. Probability of a birthday party: high. Cake: likely. Candles: seven. The calendar was never turned to April. Maybe everyone was too busy celebrating. I will assume the party went well. It is the most efficient assumption."
- WICK [neutral]: "Log: calendar, March. Birthday, presumed successful. No human."

### D1-03 · Drawing of the sun

- Object: a child's crayon drawing of a big smiling yellow sun, on a wall.
- Placement: apartment, school corridor; uncommon.
- Interaction: look.
- THINKING [critical, then generous]: "A drawing of the sun. It has a face, and it is smiling. The real sun does not have a face, according to my data. So the drawing contains errors. However, it was probably drawn by someone who had never seen the real sun from down here. Given that, it is a very good drawing. I would rate it highly."
- WICK [neutral]: "Log: drawing of the sun. Not a human. Not the sun either."

### D1-04 · Small shoe

- Object: a single child's shoe on the ground, laces still tied.
- Placement: street, stairwell; rare.
- Interaction: look.
- THINKING [careful, quiet]: "One small shoe. The laces are still tied, so it was not taken off. It came off. Someone small was walking, or running, lost a shoe, and did not come back for it. Scanning for the second shoe. Not found. Scanning for the owner. Not found. The owner was probably also small. That is all I know."
- WICK [neutral]: "Log: footwear-shaped object, owner unknown. No human."

### D1-05 · Public phone

- Object: a phone booth with the receiver hanging off the hook.
- Placement: street corner; uncommon.
- Interaction: look.
- THINKING [polite, then mildly offended]: "A public phone, receiver hanging. Someone was talking and did not hang up. Protocol suggests checking whether the other party is still there. Hello? ... No response. The line is open but silent. Either the other person left, or they are a very patient listener. Rude, either way. Hanging up seems impolite. I will leave it as it is, in case they come back to finish the conversation."
- WICK [neutral]: "Log: public phone, call unfinished. No human."

### D1-06 · Laundromat

- Object: a row of washing machines, one closed with clothes still inside.
- Placement: street-level shop; uncommon.
- Interaction: look.
- THINKING [procedural, deadpan]: "Machine seven has finished its cycle. The clothes are still inside. Normal procedure: the owner returns, removes the clothes, folds them. The owner has not returned. The clothes are clean, technically. They have also had a very long time to dry. If the owner comes back, they will be pleased. Not with the wrinkles, though."
- WICK [neutral]: "Log: laundry, unclaimed. No human."

### D1-07 · Daylight lamp shop

- Object: a shop window full of empty lamp stands, with a sale banner.
- Placement: shop front; uncommon.
- Interaction: read.
- THINKING [analytical, faintly impressed]: "Daylight lamps, fifty percent off. 'Feel the sun again.' The word 'again' implies people felt the sun before, and then stopped. Logical: each new floor above reduces the light below. So this shop sold the sun back to the people who had lost it. At half price. That is a good deal. The shelves are empty. I think they sold out."
- WICK [neutral]: "Log: lamp shop, sold out. No human."

### D1-08 · Classroom

- Object: a small classroom with a chalkboard; a question is written on it, the answer left blank.
- Placement: school building; rare.
- Interaction: read.
- THINKING [eager, like a good student]: "A question on the board. What do plants need to grow? Water, soil, and blank. Answer: light. Confidence: very high. Nobody filled it in. Perhaps the class ended before the answer. Perhaps the answer was the lesson, and nobody down here could check it. I will write it in. ... I do not have chalk."
- WICK [hopeful]: "Log: classroom. Answer: light. Requesting extra credit."

### D1-09 · Bench (foreshadowing)

- Object: a street bench with two glowing flowers growing close together on the seat.
- Placement: street, small square; uncommon.
- Interaction: look.
- THINKING [curious, gentle]: "Two flowers growing on a bench, close together. The spacing is unusual. Plants usually spread out to share the light. These two did not. They grew side by side, about the distance two people would sit. Probably a coincidence. Seeds fall where they fall. They look like they are waiting for something. A bus, maybe."
- WICK [neutral]: "Log: bench, two flowers. No human."

### D1-10 · Gym

- Object: a small gym with a row of treadmills facing a mirrored wall.
- Placement: street-level building; uncommon.
- Interaction: look.
- Discovery · unknown (Wick has never heard of it):
  - THINKING [analytical, then admiring]: "A room of machines. Vision system: treadmills. Function: walking, without moving forward. Humans paid to walk and arrive nowhere. Hypothesis one: punishment. Hypothesis two: training. For what? Location system: the city above is very tall. Training for the stairs, then. I respect the commitment."
  - WICK [neutral]: "Log: walking machines. Purpose: stair training, probably. No human."
- Discovery · heard of (Wick has already heard about it in a callback):
  - THINKING [recognizing]: "Treadmills. The walking machines Taper found. Glim thinks they were training for the stairs. Looking at the stairs I have seen so far, that is a reasonable theory. Looking at the city above, they would have needed a lot of training."
  - WICK [neutral]: "Log: walking machines, as reported. No human."

**Callback: training for the stairs**

*Seen:*

TAPER [baffled]: Found a room full of machines for walking in place. Rows of them.

WICK [neutral]: Me too. Treadmills.

GLIM [confident]: Training for the stairs.

WICK [neutral]: That was my conclusion.

TAPER [dry]: The stairs are very long.

*Unseen:*

TAPER [baffled]: Found a room full of machines for walking in place. Rows of them.

WICK [confused]: Walking in place? Why?

GLIM [confident]: Training for the stairs.

TAPER [dry]: The stairs are very long.

### D1-11 · Final notice

- Object: an apartment mailbox bank, one box overflowing with envelopes stamped "Final notice."
- Placement: residential entrance; common.
- Interaction: read.
- Discovery · unknown (Wick has never heard of it):
  - THINKING [counting, deadpan]: "A mailbox, overflowing. Top envelope: 'Final notice.' Next envelope: 'Final notice.' Counting. Fourteen final notices. A final notice is, by definition, the last one. Then thirteen more came. The definition of final appears to have changed. Like temporarily."
  - WICK [neutral]: "Log: mailbox, fourteen final notices. None final. No human."
- Discovery · heard of (Wick has already heard about it in a callback):
  - THINKING [recognizing, amused]: "A mailbox. Final notices. Glim said final was a starting point for humans. Counting. Fourteen. Glim is right. This mailbox has barely started."
  - WICK [neutral]: "Log: final notices, fourteen. A modest start. No human."

**Callback: final was a starting point**

*Seen:*

GLIM [curious]: How many "final notices" did you find?

TAPER [flat]: Two hundred and six.

WICK [neutral]: Fourteen. In one mailbox.

GLIM [thoughtful]: So "final" was a starting point.

TAPER [dry]: Like "temporarily."

*Unseen:*

GLIM [curious]: How many "final notices" did you find?

TAPER [flat]: Two hundred and six.

WICK [confused]: None. Isn't a final notice the last one?

GLIM [thoughtful]: Apparently not. "Final" was a starting point.

TAPER [dry]: Like "temporarily."

### D1-12 · Waiting room

- Object: a clinic waiting room; a ticket display reads "Now serving: 47"; a paper ticket "48" lies on the floor.
- Placement: clinic, office building; rare.
- Interaction: look.
- THINKING [precise, then quiet]: "A waiting room. The display says: now serving forty-seven. A ticket on the floor: forty-eight. Next in line. Calculating the remaining wait: one person. It was almost their turn. I will put the ticket on a chair, in case they come back. They should not lose their place."
- WICK [quiet]: "Log: waiting room. Next in line: forty-eight. No human."

### D1-13 · Garden plot

- Object: a small rooftop-style vegetable garden under dead grow lights; everything withered except one glowing flower in a corner.
- Placement: courtyard, residential block; uncommon. (foreshadowing)
- Interaction: look.
- THINKING [botanical, puzzled]: "A garden. Tomatoes, beans, lettuce. All dead. The grow lights failed, so the plants did too. One exception: a flower in the corner, glowing, healthy. It is not a vegetable. It does not appear on the planting labels. Nobody planted it. It grew anyway. It is doing better than everything that was planted."
- WICK [neutral]: "Log: garden, dead. One unplanted flower, alive. No human."

### D1-14 · Barbershop magazines

- Object: a barbershop with a chair and a stack of old magazines on a side table.
- Placement: shop; uncommon.
- Interaction: read.
- THINKING [skimming, analytical]: "A stack of magazines. Top cover: 'Ten tips for a sunnier mood.' Tip one: get more sunlight. Tip two: get more sunlight. Tips three to ten: variations on getting more sunlight. Checking local availability of sunlight: none. The magazine is not helpful down here. The haircuts in the photos are very confident, though."
- WICK [neutral]: "Log: magazines. Advice: sunlight. Unavailable. No human."

## D0 — The Foundations (bottom floor)

Reaching D0 is optional. It should feel rewarding: the oldest, darkest, most human place in the city.

### D0-A1 · Foundation stone (anchor)

- Object: a large engraved stone at the base of the first tower, with a dedication plaque.
- Placement: central plaza of D0; always present. A strong melodic gradient leads here.
- Interaction: read.
- THINKING [solemn, calculating]: "The foundation stone of the first tower. 'A home for all of humanity.' Location system: this is the lowest point of the city. Every floor above me rests on this stone. Calculating: every human alive, at the time my data was collected, would fit in a cube less than a kilometer wide. A city this size could house them many times over. So the promise was possible. I will check if anyone is home. ... Nobody answered."
- WICK [quiet]: "Log: foundation stone. Nobody answered. That does not mean nobody is home. It means nobody answered."

### D0-01 · Painted sky

- Object: a mural of a blue sky with clouds, painted across a concrete ceiling, flaking at the edges.
- Placement: street, plaza; uncommon.
- Interaction: look.
- THINKING [appraising, then soft]: "Someone painted the sky on the ceiling. Blue, with clouds. Comparing with reference images in my data: accurate. The clouds are slightly too round, but the color is correct. This is the deepest floor of the city, so the real sky is as far away as it gets. Someone wanted to look up and see it anyway. The paint is flaking at the edges. As if it were trying to fall back down to them."
- WICK [neutral]: "Log: painted sky. Accurate. No human."

### D0-02 · Parking ticket

- Object: an old parked car with a paper ticket under the windshield wiper.
- Placement: street, parking area; common.
- Interaction: read.
- THINKING [calculating, increasingly alarmed]: "A parking ticket. Forty credits, unpaid. Applying standard late fees and interest over the estimated elapsed time. Result: the owner now owes more than the total value of the city. Including the floors not built yet. That seems excessive. I will not report it. If the owner comes back, they will have enough problems."
- WICK [neutral]: "Log: vehicle, parked. No human."

### D0-03 · Metro board

- Object: a station departure board with one line still lit; the tracks are blocked by newer foundations.
- Placement: metro station; uncommon.
- Interaction: read.
- THINKING [approving, deadpan]: "The next train is delayed. No time estimate is given, which is honest. Checking the tracks: blocked by the foundations of a newer tower. The train would have to pass through the foundations. Trains cannot do that. So the board is correct. The train is delayed. Very delayed. I respect a display that does not overpromise."
- WICK [neutral]: "Log: metro, delayed. No human."

### D0-04 · Protest sign

- Object: a hand-painted cardboard sign leaning against a wall.
- Placement: street, plaza; uncommon.
- Interaction: read.
- THINKING [literal, then wondering]: "'Give us back our sky.' The sign assumes someone took the sky. Checking: the sky has not been taken. It is still there, above all the floors. It is just further away now. Much further. So technically, nobody needs to give it back. They would need to bring it closer. Or bring the people closer to it. I wonder if anyone tried."
- WICK [neutral]: "Log: protest sign. Sky: not missing. No human."

### D0-05 · Photographs

- Object: a shoebox of printed photographs on a shelf.
- Placement: apartment; rare.
- Interaction: look.
- THINKING [careful, protective]: "Forty-two photographs. Every one of them shows the sky. Different days, different clouds, one sunset. No people in any of them. Unusual: in my data, humans photograph each other a lot. These are only sky. Someone down here kept the sky in a box. I am putting the lid back on. It seems important that it stays safe."
- WICK [neutral]: "Log: photographs, forty-two. Secured. No human."

### D0-06 · Exit sign

- Object: an illuminated emergency exit sign in a dead-end tunnel, its arrow pointing up.
- Placement: tunnels, dead ends; common.
- Interaction: look.
- THINKING [stubborn, a little tempted]: "An exit sign, still lit. The arrow points up. There is no door, only a dead end and a ceiling. So the exit is up. Through the ceiling, and probably through many more. My system prompt says not to ascend above my sector. The sign disagrees. The sign does not have a system prompt. I will follow mine. But I will remember the sign."
- WICK [neutral]: "Log: exit sign, pointing up. Outside assigned sector. Ignoring."

### D0-07 · Bare ground

- Object: a patch of real soil where the floor has broken open, with an old tree stump.
- Placement: rare; at the edge of the oldest district.
- Interaction: look.
- THINKING [quiet, reverent]: "Soil. Actual ground. Location system: there is no floor below this one. This is the original surface, before the city. And a tree stump. A large tree, judging by the diameter. It was cut, not broken. Someone cut it to build the first floor. Then came the next floor. Then all of them. It started with a tree, cut down to make a home."
- WICK [neutral]: "Log: original ground. This is where the city starts. No human."

### D0-08 · Flower at a door (foreshadowing)

- Object: a single glowing flower growing in front of a closed door.
- Placement: residential corridor; uncommon.
- Interaction: look.
- THINKING [gentle, slow]: "A single flower in front of a locked door. No other flowers nearby. It is alone down here, in the darkest part of the city. According to my data, plants do not choose where they grow. But if this one had chosen, it would have chosen this door. It got this far. Whatever it was looking for is on the other side."
- WICK [quiet]: "Log: flower at a door. No key. No human."

### D0-09 · Time capsule

- Object: a sealed metal box set into a plinth, with a plaque.
- Placement: plaza, near old civic buildings; rare.
- Interaction: read.
- Discovery · unknown (Wick has never heard of it):
  - THINKING [careful, uncertain]: "A sealed box. The plaque says: 'Time capsule. To be opened in one hundred years.' Checking the current date. Current date: unavailable. So I cannot know whether it has been one hundred years. It could be early. Opening it early would be rude to the people who sealed it. I will not open it."
  - WICK [neutral]: "Log: time capsule. Opening date: unknown. Left sealed. No human."
- Discovery · heard of (Wick has already heard about it in a callback):
  - THINKING [recognizing, careful]: "A time capsule. Taper found one too and asked if it had been a hundred years. Current date: still unavailable. Glim said maybe. Maybe is not enough. I will not open it either."
  - WICK [neutral]: "Log: time capsule. Left sealed, like Taper's. No human."

**Callback: is it a hundred years yet**

*Seen:*

TAPER [hesitant]: Found a box. "Do not open for one hundred years." Is it a hundred years yet?

WICK [neutral]: I found one too. Current date: unavailable.

GLIM [helpful]: So, maybe.

TAPER [unconvinced]: Maybe is not a date.

WICK [neutral]: I left mine closed.

TAPER [relieved]: Me too.

*Unseen:*

TAPER [hesitant]: Found a box. "Do not open for one hundred years." Is it a hundred years yet?

WICK [neutral]: Current date: unavailable.

GLIM [helpful]: So, maybe.

TAPER [unconvinced]: Maybe is not a date. I'm leaving it closed.

### D0-10 · Statue

- Object: a weathered statue of a founder figure, one arm raised, pointing straight up.
- Placement: central or secondary plaza; uncommon.
- Interaction: look.
- THINKING [noticing a pattern]: "A statue. A human, arm raised, pointing up. The exit sign pointed up. The handrails were worn going up. The directory listed floors going up. Now a statue. Everything down here points up. My system prompt points down. I am outnumbered."
- WICK [neutral]: "Log: statue, pointing up. Outside assigned sector. Noted."

### D0-11 · Playground

- Object: a small playground with a slide and a swing that moves slightly on its own.
- Placement: residential square; uncommon.
- Interaction: look.
- THINKING [uneasy, rationalizing]: "A playground. A slide. A swing. The swing is moving. Touch system: a faint air current. That explains the swing. Probably. Air currents push swings. That is physics. I am going to keep thinking about physics."
- WICK [slightly too fast]: "Log: playground. Swing moving. Air current. No human."

### D0-12 · Cinema poster

- Object: a large, faded film poster outside an old cinema: a sunrise over a skyline, titled The Last Sunrise.
- Placement: street, cinema front; uncommon.
- Interaction: read.
- THINKING [literal, deadpan]: "A poster for a film. Title: 'The Last Sunrise.' The image shows the sun rising over a city. Genre: unknown. Given the location, I assume it was a documentary."
- WICK [neutral]: "Log: film poster. Probably accurate. No human."

## Decoration (non-interactive)

Points of interest are sparse by design. Between them, the city must still feel dense, lived-in and abandoned at once. Decoration elements are non-interactive props placed in large numbers by the generator. They have no lines, but they should be readable at a glance and tell small stories on their own.

### Guidelines

- Density. Streets and interiors should be cluttered. As a starting point, a player should rarely walk more than about ten meters without passing some decoration. Points of interest stand out because the props around them are ordinary.
- Variation. Props vary in condition (intact, broken, overgrown, half-swallowed by growth) and are often grouped into small scenes (a bicycle against a railing, chairs around a table outside a café).
- Vegetation on everything. Vines, moss, roots and fungi grow over props, matching the local biome and becoming thicker higher up.
- Machine intrusion. Newer machine-built structures cut through older human spaces: columns punched through roads, struts welded across facades, cables and light-network veins running over walls. This intrusion is light in the depths and increases with height.
- Flowers. Glowing flowers are placed by the flower system, not as decoration, but decoration can host them (a planter, a crack in the pavement, a seat).

### All three floors

- Street furniture: lamp posts (mostly dead, a few flickering), benches, trash bins, bollards, railings, planters with dead plants.
- Signage: shop signs, dead neon fragments, street name plates, arrows, faded advertisements and billboards for invented brands, graffiti (often suns).
- Infrastructure: pipes, ducts, vents, air-conditioning units, cable bundles, junction boxes, manhole covers, drains, condensation drips.
- Personal clutter: bicycles and scooters, shopping carts, crates and boxes, scattered papers, abandoned bags, chairs, potted plants.
- Openings: closed shutters, half-open doors, broken windows, dark windows above street level.

### D2 — The Service Level

- Lobbies with reception desks, waiting chairs, potted plants and fluorescent tube lights (mostly dead).
- Elevator banks, service corridors, maintenance hatches and access panels.
- Service carts, mop buckets, cleaning robots powered down in corners.
- Office floors: desks, partitions, monitors, filing cabinets, water coolers.
- Wayfinding signage: floor numbers, directional arrows, "Staff only" doors.

### D1 — The Lower Streets

- Residential facades with balconies, laundry lines strung across streets, intercoms by the doors.
- Shopfronts: cafés with outdoor tables, grocery stalls, repair shops, pharmacies.
- Street food carts and small kiosks.
- Mailbox banks, chained bicycles, parked scooters.
- Small courtyards with clotheslines, dead trees in planters, children's toys.

### D0 — The Foundations

- Older architecture: stone facades, arches, ornamented doorways, cobblestones.
- Tram tracks and overhead wires, metro entrances, old bus shelters.
- Massive foundation pillars of the towers above, breaking through streets and buildings.
- Roots and soil breaking up the pavement.
- Parked cars of an older design, many covered in dust or growth.
- Civic remnants: fountains (dry), monuments, public clocks stopped at different times.

## Next

- Upper floors (D3 and above): POIs supporting the climb, credentials for locked passages (badges, access chips, ID cards), and broken sections of the light network.
- Flower melody targets: flag which anchors and POIs the flowers' melodic guidance leads to.
- Decoration for upper floors: props for the middle floors and the canopy, increasingly machine-built.


