### Code permettant la mise en relation des différents dispositifs
Représentation du network

```
        [Carte STEVAL-VL53L9]
                  │
             (Bus I²C)
                  ▼
      [Arduino Uno R4 WiFi #1]  <-- Carte Maître (Traitement des données ToF)
         │               │
  (ESP-NOW/Wi-Fi)  (ESP-NOW/Wi-Fi)
         │               │
         ▼               ▼
[Arduino R4 #2]     [Arduino R4 #3]  <-- Cartes Réceptrices (Actionneurs, affichage, etc.)
```
On as besoin de
+ VL53L9CX (capteur lidar)
+ Carte STEVAL-VL53L9
+ Arduino R4 wifi*3 