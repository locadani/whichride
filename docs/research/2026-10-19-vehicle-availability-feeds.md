# Official vehicle-availability feeds for Berlin carsharing

Date: 2026-10-19. Question: can whichride show nearby *available* cars using only official,
public, legally reusable data (DECISIONS.md #4)?

## Short answer

No. None of the four operators publishes a public feed for Berlin that we could find.
- **MILES:** no public feed for Berlin. MILES does send a feed to the NRW regional data hub, but
  only for NRW cities, and access needs approval (we got HTTP 401). Not usable.
- **Bolt Drive:** Bolt publishes open GBFS feeds for some cities, with no key needed. None is listed for
  Berlin, and every Bolt feed we checked lists scooters and e-bikes, not cars. Not usable.
- **Free2move:** no public feed for Berlin. Its feed reaches the NRW hub only for the Rheinland
  zone, and access needs approval (HTTP 401). Transit app calls it a "private feed". Not usable.
- **SIXT share:** we found no feed or developer API at all. Not usable.

No aggregator republishes any of the four for Berlin under an open licence. That covers Jelbi,
Mobilithek, the MobilityData catalogue, Transitous, MobiData BW and the Berlin open data portal.
Under our rules there is currently no legal way to know which cars are near the user. One thing
could change this. A German law (IVSG reform, which absorbed the planned "Mobilitätsdatengesetz")
would make sharing operators publish data through Mobilithek, but we could not confirm that it
has passed or when it takes effect.

## MILES

- **Own feed:** none found. MILES' partner page (miles-mobility.com/en-de/partners) offers
  partnerships and affiliate programmes. It does not mention an API, GBFS or open data.
- **MobilityData systems.csv:** no MILES entry (checked the live file on 2026-10-19).
- **NRW hub (Multimodale Datendrehscheibe NRW, run by go.Rheinland):** factsheet ID 380 lists a
  MILES data supplier with zones Köln, Duisburg, Düsseldorf, Wuppertal and Solingen. It has GBFS
  2.3/3.0 endpoints at `https://mdd.gorheinland.com/gbfs-output/380/{version}/{endpoint}.json`.
  Fetching `.../380/3.0/gbfs.json` returned **HTTP 401** with "Sie haben keinen Zugriff auf Daten
  diesen Datenlieferanten ... kontaktieren Sie bitte den Betreiber der MDD." So access is restricted
  and needs approval, and Berlin is not covered. We did not find who qualifies for access.
- **Jelbi:** MILES is in Jelbi through a BVG contract. That is a partnership, not a public feed.
- **Lobbying:** the Bundestag lobby register shows MILES wants changes to the mobility data law
  ("fair terms" for real-time data sharing). This suggests MILES does not plan to publish freely.
- **Contents (size class S/M/L, fuel level):** cannot be checked, since we have no access. The MDD
  data model has a vehicle type, a battery/fuel percentage and a reserved flag, but we could not
  see what MILES actually fills in.

## Bolt Drive

- **Own feeds:** Bolt runs public GBFS feeds at `https://mds.bolt.eu/gbfs/3/{city_id}/gbfs`. No
  key is needed. We fetched Brussels (`.../336/gbfs`): HTTP 200, GBFS 3.0, with vehicle_status,
  vehicle_types, pricing plans and geofencing. Its `vehicle_types` contained only
  `scooter_standing` and `bicycle`.
- **Berlin:** systems.csv lists Bolt for Brussels, Liverpool, Austin, Washington DC, Norwegian
  cities (via Entur), and Karlsruhe, Stuttgart, Reutlingen, Basel and Zurich (via MobiData BW).
  **No Berlin entry.** We did not guess city IDs to look for an unlisted Berlin feed.
- **Cars:** the German Bolt feeds (Stuttgart, Karlsruhe via MobiData BW) contain only
  `scooter_standing`. We found no Bolt feed with `form_factor: car` anywhere.
- **Licence:** we found no Bolt licence page for its GBFS. MobiData BW republishes the BW feeds
  as open data on Mobilithek, but those are scooters outside Berlin.
- Side note: in 2025 Bolt and MILES announced that MILES cars can be booked in the Bolt app
  (vision-mobility.de). That is a commercial integration, not public data.

## Free2move (ex SHARE NOW)

- **Own feed:** no public developer page or feed found. Transit app's help article says
  "Free2Move ... provides a private feed for its service" that adapts GBFS to carsharing.
- **systems.csv:** no Free2move or SHARE NOW entry.
- **NRW hub:** factsheet ID 37 ("Free2move", updated 19.10.2026) has one zone,
  `sharenow_Rheinland`. Its GBFS endpoints are at `https://mdd.gorheinland.com/gbfs-output/37/...`.
  Fetching `.../37/3.0/gbfs.json` and `.../37/2.3/gbfs.json` returned **HTTP 401** (same message
  as for MILES). Berlin is not covered.
- **Contents:** the MDD data model offers position, vehicle type (form factor, propulsion,
  image), plate, battery/fuel percentage, reserved flag and colour. We could not see Free2move's
  actual values.

## SIXT share (lower priority)

- No GBFS/MDS feed, developer portal or systems.csv entry found. Sixt.de share pages only point to
  the SIXT app. Not on Mobilithek or the NRW hub (search found no SIXT factsheet).
- Unclear: whether SIXT still resells MILES cars. A 2023 SIXT press release says yes, but a
  current sixt.de Stuttgart page says MILES is no longer bookable in the SIXT app. Not checked
  further.

## Aggregators

- **MobilityData systems.csv** (github.com/MobilityData/gbfs): 1,261 systems. The only Berlin
  entries are nextbike Berlin, nextbike Campus Berlin-Buch, Dott Berlin and **Getaround Berlin**.
  None are our operators. The Getaround feed is public, with no key
  (`https://getaround.com/gbfs/v3/berlin/gbfs` → HTTP 200, GBFS 3.0, vehicle_status and
  vehicle_types). It is peer-to-peer car rental, not a v1 provider. The Mobility Database
  CSV export (files.mobilitydatabase.org/feeds_v2.csv) only has GTFS/GTFS-RT, so GBFS discovery
  goes through systems.csv.
- **Mobilithek** (searched through its public search API
  `POST mobilithek.info/mdp-api/mdp-msa-metadata/v2/offers/search`): searches for MILES, Bolt,
  Free2move, SHARE NOW, SIXT, GBFS and Jelbi found no offer from these operators. The only hit
  ("hvv switch Punkte Hamburg") lists Hamburg pick-up points, not vehicles. The only Berlin
  carsharing offer is cambio (station-based, licence "restricted use, fee"). The Bolt hits are
  MobiData BW's Baden-Württemberg scooter bundles.
- **Jelbi / BVG:** Jelbi is a BVG app built on Trafi. Operators join by contract (for example, 150
  cars minimum for free-floating carsharing). We found no public Jelbi API or feed. The Berlin open
  data portal (datenregister.berlin.de) has nothing for "carsharing", "gbfs" or "jelbi". "sharing"
  only returns zone and parking-area maps (dl-de-zero-2.0).
- **Berlin rules:** courts ruled (OVG Berlin-Brandenburg 2022) that free-floating carsharing is
  not "Sondernutzung". So Berlin cannot attach data duties to a permit, and we found no Berlin
  rule that forces operators to publish GBFS.
- **MobiData BW:** its GBFS manifest (106 systems) has no MILES, Free2move, SIXT or Berlin system.
  Its only Bolt systems are scooters.
- **Transitous** (feeds/de.json): its GBFS sources are BW systems plus NextbikeBerlin and
  DottBerlin. No carsharing operator in Berlin.
- **NRW hub (MDD NRW):** it does carry MILES and Free2move, but only for NRW. It is closed
  (HTTP 401) and needs approval from the operator.

## Could not confirm

- Whether the IVSG reform (Drucksache 21/2999, committee report 21/4983 of 25.03.2026) has passed,
  when it takes effect, and whether free-floating carsharing positions would become freely
  reusable on Mobilithek. If it does, this question should be checked again.
- Whether Bolt has an unlisted GBFS feed for Berlin, and whether any Bolt feed includes Bolt Drive
  cars.
- Who can get access to the NRW hub, and on what terms.
- What MILES/Free2move actually put in vehicle type (for example MILES size class), since we had
  no access.

## Sources

- MobilityData systems catalogue: https://raw.githubusercontent.com/MobilityData/gbfs/master/systems.csv (fetched 2026-10-19)
- Bolt GBFS Brussels: https://mds.bolt.eu/gbfs/3/336/gbfs and https://mds.bolt.eu/gbfs/3/336/vehicle_types
- MobiData BW GBFS manifest: https://api.mobidata-bw.de/sharing/gbfs/v3/manifest.json; Bolt Stuttgart/Karlsruhe `vehicle_types`
- Getaround Berlin GBFS: https://getaround.com/gbfs/v3/berlin/gbfs
- Mobilithek search API: https://mobilithek.info/mdp-api/mdp-msa-metadata/v2/offers/search
- MDD NRW factsheet Free2move (ID 37): https://download.vrs.de/mdd/Factsheet_37.pdf
- MDD NRW factsheet MILES (ID 380): https://download.gorheinland.com/mdd/Factsheet_380.pdf (seen via search summary only; not opened)
- MDD NRW endpoints tried: https://mdd.gorheinland.com/gbfs-output/37/3.0/gbfs.json, https://mdd.gorheinland.com/gbfs-output/380/3.0/gbfs.json (both HTTP 401)
- Transit app on carshare feeds: https://resources.transitapp.com/article/400-carshare
- MILES partners page: https://www.miles-mobility.com/en-de/partners
- MILES lobby register entry: https://www.lobbyregister.bundestag.de/suche/R004704/44874
- Bolt × MILES cooperation: https://vision-mobility.de/news/miles-bolt-kooperieren-carsharing-boomt-363155.html
- SIXT × MILES press release (2023): https://about.sixt.com/download/pressemitteilung_miles-flotte-ab-sofort-auch-ueber-die-sixt-app-buchbar/
- Transitous Germany feeds: https://raw.githubusercontent.com/public-transport/transitous/main/feeds/de.json
- Berlin open data (CKAN): https://datenregister.berlin.de/api/3/action/package_search
- OVG Berlin-Brandenburg on carsharing (2022): https://www.berlin.de/gerichte/oberverwaltungsgericht/presse/pressemitteilungen/2022/pressemitteilung.1259164.php
- Jelbi integration conditions (summary): https://www.entwicklungsstadt.de/ordnung-im-berliner-sharing-chaos-positive-bilanz-nach-sechs-jahren-mobilitaetsapp-jelbi/
- IVSG reform: https://www.verkehrsrundschau.de/nachrichten/transport-logistik/neuer-rechtsrahmen-fuer-intelligente-verkehrssysteme-beschlossen-3728357, https://dserver.bundestag.de/btd/21/049/2104983.pdf, https://carsharing.de/sites/default/files/download/2025-11/bcs-Stellungnahme%20IVSG-Novelle_0.pdf
