---
geometry: margin=2.5cm
---

# Implementace Kademlia DHT

Výsledkem zápočťáku bude knihovna, která umožní jednoduchou tvorbu aplikací postavených nad distribuovanými hash tabulkami (DHT). V principu fungování se budu držet návrhu Petara Maymounkova a Davida Maziera [1] zvaného Kademlia. Součástí knihovny bude navíc jednoduchý příklad, který bude ukazovat její použití a zároveň na něm půjde vše testovat.

Knihovna bude umožňovat ukládání a hledání párů klíč-hodnota tvořených libovolnými binárními daty. Hodnota bude podporovat variabilní délku se specifikovaným maximem (kvůli použití primárně protokolu UDP). Klíče budou fixní délky, ale knihovna bude umožňovat použití klíčů libovolně dlouhých, ze kterých pomocí hashování vytvoří klíč použitý v síti.

Pro správné fungování sítě bude klíčové, aby každý mohl komunikovat s každým. Pro adresaci proto bude použita generická adresa _multiaddr_ z projektu IPFS, která umožňuje adresovat pomocí libovolného protokolu. Primárně tak bude zajištěna podpora pro IPv4 a IPv6, ale zároveň to umožní v budoucnosti rozšířit komunikační kanály o další protokoly.

Implementace proběhne v jazyce Rust. Je to z hlavně důvodu, že bych si rád jazyk vyzkoušel. Na výkonu samotné knihovny v tomto připadě tolik nezáleží, protože nejpomalejší stejně bude síťová komunikace. Mohl bych proto volit i jiné jazyky víceméně libovolné úrovně včetně skriptovacích jazyků. Ty obvykle ale nezaručí takovou typovou bezpečnost jako Rust. A například jazyk C je zase zbytečně nízkoúrovňový s výrazně větší pravděpodobností vzniku bezpečnostních chyb.

---

#### Odkazy:

\[1\] [http://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf](http://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)

