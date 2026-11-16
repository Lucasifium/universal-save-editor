//! Raft's item ids: name and stack size (the most one slot holds), as the game's item
//! table lists them. One item per line, "id|name|stack", sorted by id.

pub const TABLE: &str = "\
1|Wooden floor|1\n\
2|Foundation|1\n\
3|Stair|1\n\
4|Wooden wall|1\n\
6|Stationary anchor|5\n\
7|Chair|5\n\
8|Simple collection net|5\n\
9|Repair|1\n\
10|Coconut|20\n\
11|Cooked beet|20\n\
12|Cooked mackerel|20\n\
13|Cooked potato|20\n\
14|Cooked shark|20\n\
15|Raw beet|20\n\
16|Raw mackerel|20\n\
17|Raw potato|20\n\
18|Raw sharkmeat|20\n\
19|Metal ore|20\n\
20|Nail|20\n\
21|Plank|20\n\
22|Rope|20\n\
23|Scrap|20\n\
24|Seaweed|20\n\
25|Palm Leaf|20\n\
31|Palm seed|20\n\
32|Metal axe|1\n\
33|Fishing rod|1\n\
34|Building Hammer|1\n\
35|Plastic hook|1\n\
36|Scrap hook|1\n\
37|Metal Spear|1\n\
38|Empty cup|1\n\
39|Cup of fresh water|1\n\
40|Cup of salt water|1\n\
42|Storage|5\n\
43|Simple grill|5\n\
45|Small crop plot|5\n\
46|Large crop plot|5\n\
47|Lantern|5\n\
48|Table|5\n\
50|Stone|20\n\
51|Giant clam|20\n\
52|Wooden spear|1\n\
53|Plastic|20\n\
54|Shark bait|1\n\
56|Throwable anchor|5\n\
59|Thatch wall|1\n\
63|Flippers|1\n\
64|Oxygen bottle|1\n\
65|Clay|20\n\
66|Sand|20\n\
67|Glass|20\n\
70|Metal ingot|20\n\
71|Smelter|5\n\
72|Wet brick|20\n\
73|Dry brick|20\n\
77|Advanced grill|5\n\
78|Advanced purifier|5\n\
79|Empty bottle|1\n\
80|Bottle of salt water|1\n\
81|Bottle of fresh water|1\n\
82|Wooden window|1\n\
83|Thatch window|1\n\
84|Wooden pillar|1\n\
85|Plank fence|1\n\
86|Rope fence|1\n\
88|Wooden door|1\n\
89|Thatch door|1\n\
91|Simple purifier|5\n\
92|Hammock|5\n\
93|Simple bed|5\n\
95|Feather|20\n\
96|Foundation armor|1\n\
97|Egg|20\n\
98|Raw drumstick|20\n\
99|Cooked drumstick|20\n\
100|Scarecrow|5\n\
101|Birds nest|5\n\
103|Paddle|1\n\
104|Red paint|600\n\
105|Blue paint|600\n\
106|Yellow paint|600\n\
107|White paint|600\n\
108|Black paint|600\n\
109|Paint brush|1\n\
110|Scrap Mechanic duck|5\n\
113|Research table|5\n\
114|Black flower|50\n\
115|Red flower|50\n\
116|Yellow flower|50\n\
117|Blue flower|50\n\
118|White flower|50\n\
119|Black seed|20\n\
120|White seed|20\n\
121|Red seed|20\n\
122|Yellow seed|20\n\
123|Blue seed|20\n\
124|Paint mill|5\n\
125|Vine goo|20\n\
126|Sail|5\n\
127|Shark head|20\n\
128|Shark trophy|5\n\
129|Raw catfish|20\n\
130|Raw salmon|20\n\
131|Raw herring|20\n\
132|Raw pomfret|20\n\
133|Cooked salmon|20\n\
134|Cooked pomfret|20\n\
135|Cooked catfish|20\n\
136|Cooked herring|20\n\
137|Cooked tilapia|20\n\
138|Raw tilapia|20\n\
139|Old shoe|5\n\
140|Small storage|5\n\
142|Streamer|5\n\
143|Raised floor|1\n\
144|Half wooden wall|1\n\
145|Half stair|1\n\
146|Half wooden pillar|1\n\
147|Half thatch wall|1\n\
148|Wooden roof|1\n\
149|Thatch roof|1\n\
150|Wooden roof corner|1\n\
151|Thatch roof corner|1\n\
152|Triangular wooden wall|1\n\
153|Triangular thatch wall|1\n\
154|Bolt|20\n\
155|Hinge|20\n\
156|Lucky cat|5\n\
157|Candle Bottle|5\n\
158|Rug|5\n\
159|Clock|5\n\
160|Inverted wooden roof corner|1\n\
161|Inverted thatch roof corner|1\n\
162|Medium crop plot|5\n\
163|Watermelon|20\n\
164|Mango|20\n\
165|Pineapple|20\n\
166|Watermelon seed|20\n\
167|Mango seed|20\n\
168|Pineapple seed|20\n\
169|Metal fishing rod|1\n\
170|Binoculars|1\n\
171|Calendar|5\n\
172|Sign|5\n\
173|Receiver|5\n\
175|Antenna|5\n\
176|Simple Battery|1\n\
177|Copper Ingot|20\n\
178|Circuit board|20\n\
179|Copper|20\n\
181|Blueprint: Receiver|1\n\
182|Blueprint: Antenna|1\n\
184|Ladder|5\n\
185|Basic bow|1\n\
186|Stone arrow|30\n\
187|Shelf|5\n\
188|Stone axe|1\n\
189|Triangular foundation|1\n\
191|Triangular wooden floor|1\n\
193|Triangular raised floor|1\n\
194|Cookingpot|5\n\
195|Large trophy board|5\n\
196|Medium trophy board|5\n\
197|Small trophy board|5\n\
198|Cave mushroom|20\n\
199|Clay bowl|20\n\
200|Red berries|20\n\
201|Silver algae|20\n\
202|Vegetable soup|5\n\
203|Leftovers|5\n\
204|Recipe: Vegetable soup|20\n\
205|Screecher head|20\n\
206|Poison-puffer head|20\n\
207|Metal arrow|30\n\
208|Simple fish stew|5\n\
210|Coconut chicken|5\n\
211|Steak with jam|5\n\
212|Head broth|5\n\
213|Mushroom omelette|5\n\
215|Shark dinner|5\n\
216|BBQ|5\n\
217|Sushi|5\n\
218|Recipe: Simple fish stew|20\n\
220|Recipe: Coconut chicken|20\n\
221|Recipe: Steak with jam|20\n\
222|Recipe: Head broth|20\n\
223|Recipe: Mushroom omelette|20\n\
225|Recipe: Shark dinner|20\n\
226|Recipe: BBQ|20\n\
227|Recipe: Sushi|20\n\
235|Pilot helmet|5\n\
237|Grass plot|20\n\
238|Shovel|1\n\
239|Dirt|20\n\
240|Bucket|20\n\
241|Bucket of milk|1\n\
242|Shears|1\n\
243|Wool|20\n\
244|Net launcher|1\n\
245|Net canister|20\n\
246|Leather|20\n\
247|Explosive goo|20\n\
248|Healing salve|20\n\
249|Leather helmet|1\n\
250|Leather body armor|1\n\
251|Leather greaves|1\n\
252|Backpack|1\n\
253|Warthog head|20\n\
254|Explosive powder|20\n\
255|Wooden gate|1\n\
256|Thatch gate|1\n\
257|Nametag|1\n\
259|Sprinkler|5\n\
260|Llama|20\n\
261|Clucker|20\n\
262|Goat|20\n\
263|Radio|5\n\
264|Captain's hat|5\n\
265|Engine|5\n\
268|Steering Wheel|5\n\
270|Blueprint: Head light|1\n\
271|Head light|1\n\
272|Blueprint: Engine|1\n\
273|Blueprint: Steering wheel|1\n\
274|Biofuel refiner|5\n\
275|Machete|1\n\
276|Honey|20\n\
277|Fuel pipe|20\n\
278|Fuel tank|20\n\
279|Blueprint: Machete|1\n\
280|Blueprint: Fuel pipe|1\n\
281|Blueprint: Fuel tank|1\n\
282|Blueprint: Biofuel refiner|1\n\
283|Pinecone|20\n\
284|Birch seed|20\n\
285|Bear head|20\n\
286|Biofuel|20\n\
287|Mama bear head|20\n\
288|Cooked meat|20\n\
289|Raw meat|20\n\
290|Beehive|5\n\
291|Sweep net|1\n\
292|Bee jar|20\n\
293|Battery charger|5\n\
294|Honeycomb|20\n\
295|Good healing salve|20\n\
296|Large Storage|5\n\
297|Blueprint: Large storage|1\n\
298|Blueprint: Metal detector|1\n\
299|Blueprint: Battery charger|1\n\
300|Metal detector|1\n\
301|Titanium ore|20\n\
302|Major Tom|5\n\
306|White screecher head|20\n\
307|Titanium ingot|20\n\
308|Zipline|5\n\
309|Blueprint: Zipline tool|1\n\
310|Firework|50\n\
311|Zipline tool|1\n\
312|Tiki piece|5\n\
313|Tiki piece|5\n\
314|Tiki piece|5\n\
315|Tiki piece|5\n\
316|Tiki pole|5\n\
317|Tiki mask|5\n\
318|Toy robot|5\n\
319|Blueprint: Firework|1\n\
320|Developer hat|5\n\
321|Mayor hat|5\n\
322|Electric purifier|5\n\
323|EDM cassette|1\n\
324|Pop cassette|1\n\
325|Rock cassette|1\n\
326|Classical cassette|1\n\
327|Banana|20\n\
328|Strawberry|20\n\
329|Strawberry seed|20\n\
330|Banana seed|20\n\
331|Man And Moon|5\n\
332|The Pot|5\n\
333|After The Rain|5\n\
334|Octohenge|5\n\
335|Neighbors Dog|5\n\
336|Real Self Portrait|5\n\
337|What Even Is?|5\n\
338|Foul Fogs|5\n\
339|Blueprint: Zipline|1\n\
340|Blueprint: Electric purifier|1\n\
341|Water pipe|20\n\
342|Water tank|20\n\
343|Blueprint: Water tank|1\n\
344|Blueprint: Water pipe|1\n\
345|Piano|5\n\
346|Tangaroa plant|5\n\
347|Tangaroa plant|5\n\
348|Tangaroa plant|5\n\
349|Tangaroa plant|5\n\
350|Tangaroa plant|5\n\
351|Tangaroa plant|5\n\
352|Tangaroa painting|5\n\
353|Tangaroa painting|5\n\
354|Tangaroa painting|5\n\
355|Tangaroa painting|5\n\
356|Tangaroa painting|5\n\
357|Elevator cassette|1\n\
358|OP weapon|1\n\
359|Golden toy robot|5\n\
360|Compass|1\n\
361|Wind turbine|5\n\
362|Engine controls|5\n\
363|Recycler|5\n\
364|Trash cube|20\n\
365|Construction helmet|5\n\
366|Juicer|5\n\
367|Drinking glass|20\n\
368|Anglerfish head|20\n\
369|Rhino shark trophy|20\n\
371|Blueprint: Wind turbine|1\n\
372|Blueprint: Recycler|1\n\
373|Blueprint: Engine controls|1\n\
381|Polar bear head|20\n\
382|Solid foundation|1\n\
383|Triangular solid foundation|1\n\
384|Solid wooden floor|1\n\
385|Triangular solid wooden floor|1\n\
386|Solid wooden fence|1\n\
387|Triangular solid foundation|1\n\
388|Triangular solid wooden floor|1\n\
389|Trashcan|5\n\
390|Bookpile|5\n\
391|Quill|5\n\
392|Bookpile|5\n\
393|Chair|5\n\
394|Table|5\n\
395|Sofa|5\n\
396|Raised solid floor|1\n\
397|Triangular raised solid floor|1\n\
398|Solid ladder|1\n\
399|Solid wooden pillar|1\n\
400|Solid half wooden pillar|1\n\
401|Solid wooden roof corner|1\n\
402|Inverted solid wooden roof corner|1\n\
403|Solid wooden roof|1\n\
404|Solid stair|1\n\
405|Solid half stair|1\n\
406|Solid wooden door|1\n\
407|Solid wooden gate|1\n\
408|Triangular solid wooden wall|1\n\
409|Solid wooden wall|1\n\
410|Solid half wooden wall|1\n\
411|Solid wooden window|1\n\
412|Cutlery|5\n\
413|Candlestick|5\n\
414|Candlestick|5\n\
415|Mug|5\n\
416|Table|5\n\
417|Table|5\n\
418|Chair|5\n\
419|Chair|5\n\
420|Table|5\n\
421|Pyramid wooden wall|1\n\
422|Pyramid thatch wall|1\n\
423|Pyramid solid wooden wall|1\n\
424|Wooden roof endcap|1\n\
425|Thatch roof endcap|1\n\
426|Solid wooden roof endcap|1\n\
427|Wooden roof pyramid|1\n\
428|Thatch roof pyramid|1\n\
429|Solid wooden roof pyramid|1\n\
430|Wooden double roof|1\n\
431|Thatch double roof|1\n\
432|Solid wooden double roof|1\n\
433|World globe|5\n\
434|Metal lantern|5\n\
435|Shelf|5\n\
436|Shelf|5\n\
437|Rug|5\n\
438|Rug|5\n\
439|Rug|5\n\
440|Sofa|5\n\
441|Sofa|5\n\
442|Clock|5\n\
443|Triangular wooden wall|1\n\
444|Triangular thatch wall|1\n\
445|Triangular solid wooden wall|1\n\
446|Curtain|5\n\
447|Curtain|5\n\
448|Bed|5\n\
449|Rose|5\n\
450|Sunflower|5\n\
451|Monstera|5\n\
452|Hibiscus|5\n\
453|Cactus|5\n\
454|Aloe Vera|5\n\
455|Table|5\n\
456|Cupboard|5\n\
457|Cupboard|5\n\
458|Cupboard|5\n\
459|Table|5\n\
460|Shelf|5\n\
461|Firewood rack|5\n\
462|Fireplace|5\n\
463|Fire basket|5\n\
465|Triangular raised solid floor|1\n\
466|String light|5\n\
467|String light|5\n\
468|Decoration package|20\n\
469|Painting|5\n\
470|Painting|5\n\
471|Painting|5\n\
472|Painting|5\n\
473|Wooden bathtub|5\n\
474|Outhouse|5\n\
475|Flag|5\n\
476|Flag|5\n\
477|Flag|5\n\
478|Flag|5\n\
479|Flag|5\n\
480|Flag|5\n\
481|Flag|5\n\
482|Paper bundle|5\n\
483|Open book|5\n\
484|Motivational quote|5\n\
485|Motivational quote|5\n\
486|Motivational quote|5\n\
487|Motivational quote|5\n\
488|Wooden T-junction|1\n\
489|Thatch T-junction|1\n\
490|Solid wooden T-junction|1\n\
491|Half wooden window|1\n\
492|Half thatch window|1\n\
493|Solid half wooden window|1\n\
494|Half ladder|1\n\
495|Solid half ladder|1\n\
496|Table|5\n\
497|Wooden X-junction|1\n\
498|Thatch X-junction|1\n\
499|Solid wooden X-junction|1\n\
500|Solid wooden L-junction|1\n\
501|Thatch L-junction|1\n\
502|Wooden L-junction|1\n\
503|Advanced collection net|5\n\
504|Hearty stew|5\n\
508|Redbeet shot|5\n\
509|Spicy pineberry|5\n\
510|Simple smoothie|5\n\
511|Mangonana|5\n\
512|Red melon|5\n\
513|Silver smoothie|5\n\
514|Coconut beat|5\n\
515|Strawberry colada|5\n\
516|Recipe: Hearty stew|20\n\
520|Recipe: Spicy pineberry|20\n\
521|Recipe: Redbeet shot|20\n\
522|Recipe: Simple smoothie|20\n\
523|Recipe: Coconut beat|20\n\
524|Recipe: Strawberry colada|20\n\
525|Recipe: Mangonana|20\n\
526|Recipe: Red melon|20\n\
527|Recipe: Silver smoothie|20\n\
528|Juniper|20\n\
529|Turmeric|20\n\
530|Chili|20\n\
531|Recipe: Salmon salad|20\n\
532|Recipe: Catfish deluxe|20\n\
533|Salmon salad|5\n\
534|Catfish deluxe|5\n\
535|Advanced Battery|1\n\
536|Hyena head|20\n\
537|Alpha's head|20\n\
538|Advanced stationary anchor|5\n\
539|Advanced biofuel refiner|5\n\
540|Electric Smelter|5\n\
541|Horizontal wooden pillar|1\n\
542|Horizontal half wooden pillar|1\n\
543|Horizontal solid wooden pillar|1\n\
544|Horizontal solid half wooden pillar|1\n\
545|Detail plank|20\n\
546|Replace with wood|1\n\
547|Replace with thatch|1\n\
548|Replace with solid wood|1\n\
549|Electric zipline tool|1\n\
550|Leftovers|5\n\
551|Blueprint: Advanced stationary anchor|1\n\
552|Blueprint: Advanced battery|1\n\
553|Blueprint: Electric smelter|1\n\
554|Blueprint: Advanced biofuel refiner|1\n\
555|Blueprint: Electric zipline tool|1\n\
556|Blueprint: Big backpack|1\n\
557|Titanium arrow|30\n\
558|Titanium sword|1\n\
559|Titanium axe|1\n\
560|Titanium hook|1\n\
561|Blueprint: Titanium tools|1\n\
562|Big backpack|1\n\
564|Trade coin|20\n\
565|Advanced Scarecrow|5\n\
566|Tic-tac-toe|5\n\
567|GlowShrimp|20\n\
568|Netnibbler|20\n\
569|Sharp Carp|20\n\
570|Trap Snapper|20\n\
571|Foamspitter|20\n\
572|Pufferlisk|20\n\
573|Pentapus|20\n\
574|LionFish|20\n\
575|Lunarfish|20\n\
576|Sharkeater|20\n\
577|Rainbow Lobster|20\n\
578|Pink Pike|20\n\
579|Simple fishing bait|40\n\
580|Advanced fishing bait|40\n\
581|Expert fishing bait|40\n\
582|Foundation counter|5\n\
583|Air horn|5\n\
584|Honk horn|5\n\
585|Viking horn|5\n\
587|Short draw bridge|5\n\
588|Long draw bridge|5\n\
589|Utopia plant|5\n\
590|Utopia plant|5\n\
591|Utopia plant|5\n\
592|Utopia light bottle|5\n\
593|Utopia light bottle|5\n\
594|Utopia herbs|5\n\
595|Utopia herbs|5\n\
596|Utopia herbs|5\n\
597|Utopia herbs|5\n\
598|Utopia herbs|5\n\
599|Utopia herbs|5\n\
600|Utopia dream catcher|5\n\
601|Utopia decoration|5\n\
602|Utopia decoration|5\n\
603|Utopia decoration|5\n\
604|Utopia decoration|5\n\
605|Utopia decoration|5\n\
606|Utopia bottles|5\n\
607|Temperance painting|5\n\
608|Temperance painting|5\n\
609|Temperance painting|5\n\
610|Temperance plant|5\n\
611|Temperance plant|5\n\
612|Temperance plant|5\n\
613|Temperance rug|5\n\
614|Temperance rug|5\n\
615|Temperance cutlery|5\n\
616|Temperance mug|5\n\
617|Advanced head light|1\n\
618|Blueprint: Advanced head light|1\n\
619|Spiral stair case|5\n\
620|Half spiral staircase|5\n\
621|Chef's hat|5\n\
622|Disguise glasses|5\n\
623|Sunglasses|5\n\
624|Sailor hat|5\n\
625|Fishing hat|5\n\
626|Diving helmet|5\n\
627|Pirate hat|5\n\
628|Empty canteen|1\n\
629|Canteen of salt water|1\n\
630|Canteen of fresh water|1\n\
631|Wardrobe|5\n\
632|Blueprint: Canteen|1\n\
633|Blueprint: Detail plank|1\n\
634|Advanced small crop plot|5\n\
635|Advanced large crop plot|5\n\
636|Advanced medium crop plot|5\n\
637|Blueprint: Advanced small crop plot|1\n\
638|Blueprint: Advanced medium crop plot|1\n\
639|Blueprint: Advanced large crop plot|1\n\
640|Fridge|5\n\
641|Surfboard|5\n\
642|Electric grill|5\n\
643|Blueprint: Electric grill|1\n\
644|Titanium tools|1\n\
645|Trading post cassette|1\n\
";

/// `(id, name, stack)` of every item, sorted by id.
pub fn items() -> &'static [(i32, &'static str, i32)] {
    static ITEMS: std::sync::OnceLock<Vec<(i32, &'static str, i32)>> = std::sync::OnceLock::new();
    ITEMS.get_or_init(|| {
        TABLE
            .lines()
            .filter_map(|l| {
                let mut p = l.splitn(3, '|');
                Some((p.next()?.parse().ok()?, p.next()?, p.next()?.parse().ok()?))
            })
            .collect()
    })
}
