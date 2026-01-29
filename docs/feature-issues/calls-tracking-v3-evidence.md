# Calls Tracking v3 - Evidence File

**Generated**: 2026-01-29
**Test codebase**: /Users/michal/dev/mms/usynxissetup/app
**Total values**: 4001
**Total calls**: 3489

## Summary Statistics

### Value Kinds
| Kind | Count |
|------|-------|
| local | 1663 |
| literal | 1447 |
| parameter | 581 |
| constant | 310 |

### Call Kinds
| Kind | Count |
|------|-------|
| method | 1801 |
| access | 951 |
| constructor | 388 |
| function | 221 |
| access_array | 56 |
| method_static | 41 |
| coalesce | 20 |
| ternary_full | 6 |
| access_nullsafe | 2 |
| method_nullsafe | 2 |
| ternary | 1 |

### Call Kind Types
| Kind Type | Count |
|-----------|-------|
| invocation | 2453 |
| access | 1009 |
| operator | 27 |

### Assignment Tracking
| Tracking | Count |
|----------|-------|
| source_call_id | 476 |
| source_value_id | 147 |

---

## Evidence Cases (60+ examples)

### Case 1: Parameter Values (kind: parameter)

Parameters are tracked as values with SCIP symbols and types.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:34:31","kind":"parameter","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().($input)","type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Input/InputInterface#","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":34,"col":31}},{"id":"src/Command/TestSynchronizedItemCommand.php:34:39","kind":"parameter","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().($output)","type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Output/OutputInterface#","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":34,"col":39}},{"id":"src/DataFixtures/AppFixtures.php:17:8","kind":"parameter","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/AppFixtures#load().($manager)","type":"scip-php composer doctrine/persistence b9c49ad3558bb77ef973f4e173f2e9c2eca9be09 Doctrine/Persistence/ObjectManager#","location":{"file":"src/DataFixtures/AppFixtures.php","line":17,"col":8}}]
```

---

### Case 2: Local Variable Definitions with source_call_id

Local variables assigned from method calls track the source.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:34:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$io@34","type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":34,"col":8},"source_call_id":"src/Command/TestSynchronizedItemCommand.php:34:14"},{"id":"src/Command/TestSynchronizedItemCommand.php:37:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$data@37","type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":37,"col":8},"source_call_id":"src/Command/TestSynchronizedItemCommand.php:37:16"},{"id":"src/Command/TestSynchronizedItemCommand.php:40:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$estateUuid@40","type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":40,"col":8},"source_call_id":"src/Command/TestSynchronizedItemCommand.php:40:22"}]
```

---

### Case 3: Local Variable Definitions with source_value_id

Local variables assigned from other values track the source.

```json
[{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:117:37","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().local$rateType@117","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":117,"col":37},"source_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:117:17"},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:117:37","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().local$rateType@117","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":117,"col":37},"source_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:117:17"},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:127:37","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().local$roomType@127","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":127,"col":37},"source_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:127:17"}]
```

---

### Case 4: Local Variable Usages (same symbol)

Usages of local variables reference the same symbol as definition.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:38:27","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$data@37","type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":38,"col":27}},{"id":"src/Command/TestSynchronizedItemCommand.php:39:27","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$data@37","type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":39,"col":27}},{"id":"src/Command/TestSynchronizedItemCommand.php:40:73","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$data@37","type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":40,"col":73}}]
```

---

### Case 5: Unique Local Symbols with @line

Local variable symbols include scope and line number for uniqueness.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:34:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$io@34","type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":34,"col":8},"source_call_id":"src/Command/TestSynchronizedItemCommand.php:34:14"},{"id":"src/Command/TestSynchronizedItemCommand.php:37:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$data@37","type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":37,"col":8},"source_call_id":"src/Command/TestSynchronizedItemCommand.php:37:16"},{"id":"src/Command/TestSynchronizedItemCommand.php:38:27","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$data@37","type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":38,"col":27}}]
```

---

### Case 6: Literal Values

Literals are tracked as values without symbols.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:37:29","kind":"literal","symbol":null,"type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":37,"col":29}},{"id":"src/Command/TestSynchronizedItemCommand.php:38:33","kind":"literal","symbol":null,"type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":38,"col":33}},{"id":"src/Command/TestSynchronizedItemCommand.php:39:33","kind":"literal","symbol":null,"type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":39,"col":33}}]
```

---

### Case 7: Constant Values

Constants are values (not calls) with proper SCIP symbols.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:91:84","kind":"constant","symbol":"scip-php composer php 8.4.17 DateTime#ATOM.","type":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":91,"col":84}},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:111:12","kind":"constant","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Entity/ItemType#ESTATE.","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":111,"col":12}},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:112:12","kind":"constant","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#BOHO_ESTATE_ID.","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":112,"col":12}}]
```

---

### Case 8: Method Calls (kind_type: invocation)

Method calls have receiver_value_id and arguments.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:84:8","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().","return_type":"scip-php php builtin . void#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:84:8","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":84,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().($message)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:84:18","value_expr":"'Dispatching EstateCreatedMessage...'"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:85:8","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().","return_type":"scip-php php builtin . void#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:85:8","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":85,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().($message)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:85:18","value_expr":"\"Estate UUID: {$estateUuid}\""}]},{"id":"src/Command/TestSynchronizedItemCommand.php:86:8","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().","return_type":"scip-php php builtin . void#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:86:8","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":86,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().($message)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:86:18","value_expr":"\"Hotel Code: {$hotelCode}\""}]}]
```

---

### Case 9: Static Method Calls

Static method calls have no receiver_value_id.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:29:8","kind":"method_static","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#__construct().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Command/Command#__construct().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":29,"col":8},"arguments":[]},{"id":"src/Exception/ResourceNotSynchronizedException.php:13:8","kind":"method_static","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Exception/ResourceNotSynchronizedException#__construct().","callee":"scip-php composer php 8.4.17 Exception#__construct().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Exception/ResourceNotSynchronizedException.php","line":13,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer php 8.4.17 Exception#__construct().($message)","value_type":null,"value_id":"src/Exception/ResourceNotSynchronizedException.php:14:12","value_expr":"sprintf('Resource of type: %s and ID %s is not synchronized.', $resource, $identifier)"}]},{"id":"src/Exception/SynxisConfigurationException.php:12:8","kind":"method_static","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Exception/SynxisConfigurationException#__construct().","callee":"scip-php composer php 8.4.17 Exception#__construct().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Exception/SynxisConfigurationException.php","line":12,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer php 8.4.17 Exception#__construct().($message)","value_type":null,"value_id":"src/Exception/SynxisConfigurationException.php:12:28","value_expr":"$message"}]}]
```

---

### Case 10: Nullsafe Method Calls

Nullsafe method calls (?->) tracked correctly.

```json
[{"id":"src/Service/Synxis/SynxisCodeProvider.php:41:15","kind":"method_nullsafe","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Service/Synxis/SynxisCodeProvider#getExisting().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Entity/SynchronizedItem#getCode().","return_type":null,"receiver_value_id":"src/Service/Synxis/SynxisCodeProvider.php:41:15","location":{"file":"src/Service/Synxis/SynxisCodeProvider.php","line":41,"col":15},"arguments":[]},{"id":"src/Synxis/Api/Model/Payload/Hotel/DateRange.php:34:25","kind":"method_nullsafe","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Synxis/Api/Model/Payload/Hotel/DateRange#jsonSerialize().","callee":"scip-php composer php 8.4.17 DateTime#format().","return_type":"scip-php php builtin . string#","receiver_value_id":"src/Synxis/Api/Model/Payload/Hotel/DateRange.php:34:32","location":{"file":"src/Synxis/Api/Model/Payload/Hotel/DateRange.php","line":34,"col":25},"arguments":[{"position":0,"parameter":"scip-php composer php 8.4.17 DateTime#format().($format)","value_type":null,"value_id":"src/Synxis/Api/Model/Payload/Hotel/DateRange.php:34:49","value_expr":"'Y-m-d'"}]}]
```

---

### Case 11: Function Calls

Function calls tracked with arguments.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:37:16","kind":"function","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer php 8.4.17 random_bytes().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":37,"col":16},"arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:37:29","value_expr":"16"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:38:23","kind":"function","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer php 8.4.17 ord().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":38,"col":23},"arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:38:27","value_expr":"$data[6]"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:38:19","kind":"function","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer php 8.4.17 chr().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":38,"col":19},"arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":null,"value_expr":"ord($data[6]) & 0xf | 0x40"}]}]
```

---

### Case 12: Constructor Calls

Constructor calls (new Foo()) with return_type = class symbol.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:34:14","kind":"constructor","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#__construct().","return_type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#","receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":34,"col":14},"arguments":[{"position":0,"parameter":null,"value_type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Input/InputInterface#","value_id":"src/Command/TestSynchronizedItemCommand.php:34:31","value_expr":"$input"},{"position":1,"parameter":null,"value_type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Output/OutputInterface#","value_id":"src/Command/TestSynchronizedItemCommand.php:34:39","value_expr":"$output"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:51:23","kind":"constructor","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer php 8.4.17 DateTime#__construct().","return_type":"scip-php composer php 8.4.17 DateTime#","receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":51,"col":23},"arguments":[]},{"id":"src/Command/TestSynchronizedItemCommand.php:62:29","kind":"constructor","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#__construct().","return_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#","receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":62,"col":29},"arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#__construct().($latitude)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:63:30","value_expr":"45.5236"},{"position":1,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#__construct().($longitude)","value_type":null,"value_id":null,"value_expr":"-122.675"}]}]
```

---

### Case 13: Property Access (kind_type: access)

Property access is a CALL (not value) with kind: access.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:89:15","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#$messageBus.","return_type":"scip-php composer symfony/messenger 51e2b8b6a14b78ad7db60ef5f195ae893c16b9cc Symfony/Component/Messenger/MessageBusInterface#","receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":89,"col":15},"arguments":[]},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:152:15","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#dispatchEstateCreatedEvent().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#$messageBus.","return_type":"scip-php composer symfony/messenger 51e2b8b6a14b78ad7db60ef5f195ae893c16b9cc Symfony/Component/Messenger/MessageBusInterface#","receiver_value_id":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":152,"col":15},"arguments":[]},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:166:15","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#dispatchRateTypeCreatedEvent().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#$messageBus.","return_type":"scip-php composer symfony/messenger 51e2b8b6a14b78ad7db60ef5f195ae893c16b9cc Symfony/Component/Messenger/MessageBusInterface#","receiver_value_id":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":166,"col":15},"arguments":[]}]
```

---

### Case 14: Nullsafe Property Access

Nullsafe property access (?->) uses access_nullsafe kind.

```json
[{"id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:54:58","kind":"access_nullsafe","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#createFromEstateMessage().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#$latitude.","return_type":null,"receiver_value_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:54:44","location":{"file":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php","line":54,"col":58},"arguments":[]},{"id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:55:58","kind":"access_nullsafe","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#createFromEstateMessage().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#$longitude.","return_type":null,"receiver_value_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:55:44","location":{"file":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php","line":55,"col":58},"arguments":[]}]
```

---

### Case 15: Array Access

Array access uses access_array with key_id.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:38:27","kind":"access_array","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"","return_type":null,"receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:38:27","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":38,"col":27},"arguments":[],"key_id":"src/Command/TestSynchronizedItemCommand.php:38:33"},{"id":"src/Command/TestSynchronizedItemCommand.php:39:27","kind":"access_array","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"","return_type":null,"receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:39:27","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":39,"col":27},"arguments":[],"key_id":"src/Command/TestSynchronizedItemCommand.php:39:33"},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:120:16","kind":"access_array","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().","callee":"","return_type":null,"receiver_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:120:16","location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":120,"col":16},"arguments":[],"key_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:120:26"}]
```

---

### Case 16: Coalesce Operator

Null coalesce operator with left_id/right_id.

```json
[{"id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:54:20","kind":"coalesce","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#createFromEstateMessage().","callee":"scip-php operator . coalesce#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php","line":54,"col":20},"arguments":[],"left_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:54:58","right_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:54:70"},{"id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:55:20","kind":"coalesce","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#createFromEstateMessage().","callee":"scip-php operator . coalesce#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php","line":55,"col":20},"arguments":[],"left_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:55:58","right_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:55:71"},{"id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:76:16","kind":"coalesce","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory#createFromEstateTaxRateMessage().","callee":"scip-php operator . coalesce#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php","line":76,"col":16},"arguments":[],"left_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:76:26","right_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:76:36"}]
```

---

### Case 17: Ternary Operator (full)

Full ternary with condition_id, true_id, false_id.

```json
[{"id":"src/Logger/ApiErrorLogger.php:34:22","kind":"ternary_full","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Logger/ApiErrorLogger#log().","callee":"scip-php operator . ternary#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Logger/ApiErrorLogger.php","line":34,"col":22},"arguments":[],"true_id":"src/Logger/ApiErrorLogger.php:34:54","false_id":"src/Logger/ApiErrorLogger.php:34:76"},{"id":"src/MessageHandler/EstateMessageHandler.php:43:21","kind":"ternary_full","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#handle().","callee":"scip-php operator . ternary#","return_type":null,"receiver_value_id":null,"location":{"file":"src/MessageHandler/EstateMessageHandler.php","line":43,"col":21},"arguments":[],"false_id":"src/MessageHandler/EstateMessageHandler.php:43:94"},{"id":"src/MessageHandler/Estate/Fee/AbstractFeeOrTaxHandler.php:105:18","kind":"ternary_full","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/Estate/Fee/AbstractFeeOrTaxHandler#addTaxToHotel().","callee":"scip-php operator . ternary#","return_type":null,"receiver_value_id":null,"location":{"file":"src/MessageHandler/Estate/Fee/AbstractFeeOrTaxHandler.php","line":105,"col":18},"arguments":[],"true_id":"src/MessageHandler/Estate/Fee/AbstractFeeOrTaxHandler.php:105:74","false_id":"src/MessageHandler/Estate/Fee/AbstractFeeOrTaxHandler.php:105:101"}]
```

---

### Case 18: Ternary Operator (elvis)

Elvis operator (?:) with left_id/right_id.

```json
[{"id":"tests/Synxis/Api/Exception/ApiRequestExceptionTest.php:20:12","kind":"ternary","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Tests/Synxis/Api/Exception/ApiRequestExceptionTest#testStoresAndReturnsData().","callee":"scip-php operator . elvis#","return_type":null,"receiver_value_id":null,"location":{"file":"tests/Synxis/Api/Exception/ApiRequestExceptionTest.php","line":20,"col":12},"arguments":[],"left_id":"tests/Synxis/Api/Exception/ApiRequestExceptionTest.php:20:12","right_id":"tests/Synxis/Api/Exception/ApiRequestExceptionTest.php:20:48"}]
```

---

### Case 19: Arguments with value_id

Arguments have value_id (not value_call_id) referencing either value or call.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:34:14","kind":"constructor","arguments":[{"position":0,"parameter":null,"value_type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Input/InputInterface#","value_id":"src/Command/TestSynchronizedItemCommand.php:34:31","value_expr":"$input"},{"position":1,"parameter":null,"value_type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Output/OutputInterface#","value_id":"src/Command/TestSynchronizedItemCommand.php:34:39","value_expr":"$output"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:37:16","kind":"function","arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:37:29","value_expr":"16"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:38:23","kind":"function","arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:38:27","value_expr":"$data[6]"}]}]
```

---

### Case 20: Calls with return_type

Calls track return_type for data flow.

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:34:14","kind":"constructor","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#__construct().","return_type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#"},{"id":"src/Command/TestSynchronizedItemCommand.php:51:23","kind":"constructor","callee":"scip-php composer php 8.4.17 DateTime#__construct().","return_type":"scip-php composer php 8.4.17 DateTime#"},{"id":"src/Command/TestSynchronizedItemCommand.php:62:29","kind":"constructor","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#__construct().","return_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#"}]
```

---

### Case 21: Property Access Chains

Property chains use receiver_value_id to link.

```json
[{"id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:39:35","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#createFromEstateMessage().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateMessage#$contact.","return_type":"scip-php synthetic union . EstateContact|null#","receiver_value_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:39:20","location":{"file":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php","line":39,"col":35},"arguments":[]},{"id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:39:44","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#createFromEstateMessage().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#$email.","return_type":null,"receiver_value_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:39:35","location":{"file":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php","line":39,"col":44},"arguments":[]},{"id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:40:35","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#createFromEstateMessage().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateMessage#$newChannelNotificationEmail.","return_type":null,"receiver_value_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:40:20","location":{"file":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php","line":40,"col":35},"arguments":[]}]
```

---

### Case 22: Version 3.0

Schema version is 3.0.

```json
{"version":"3.0","project_root":"/Users/michal/dev/mms/usynxissetup/app"}
```

---

### Case 23-25: More Parameter Examples

```json
[{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:110:8","kind":"parameter","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().($manager)","type":"scip-php composer doctrine/persistence b9c49ad3558bb77ef973f4e173f2e9c2eca9be09 Doctrine/Persistence/ObjectManager#","location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":110,"col":8}},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:118:12","kind":"parameter","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().($manager)","type":"scip-php composer doctrine/persistence b9c49ad3558bb77ef973f4e173f2e9c2eca9be09 Doctrine/Persistence/ObjectManager#","location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":118,"col":12}},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:128:12","kind":"parameter","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().($manager)","type":"scip-php composer doctrine/persistence b9c49ad3558bb77ef973f4e173f2e9c2eca9be09 Doctrine/Persistence/ObjectManager#","location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":128,"col":12}}]
```

---

### Case 26-28: More Local with source_call_id

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:44:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$message@44","type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateCreatedMessage#","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":44,"col":8},"source_call_id":"src/Command/TestSynchronizedItemCommand.php:44:19"},{"id":"src/EventListener/MessageValidationFailedEventListener.php:22:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/EventListener/MessageValidationFailedEventListener#onMessageValidationFailed().local$exception@22","type":"scip-php composer php 8.4.17 Throwable#","location":{"file":"src/EventListener/MessageValidationFailedEventListener.php","line":22,"col":8},"source_call_id":"src/EventListener/MessageValidationFailedEventListener.php:22:21"},{"id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:56:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory#createFromEstateTaxRateMessage().local$type@56","type":null,"location":{"file":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php","line":56,"col":8},"source_call_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:56:38"}]
```

---

### Case 29-31: More Local with source_value_id

```json
[{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:127:37","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().local$roomType@127","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":127,"col":37},"source_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:127:17"},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:141:37","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().local$rateType@141","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":141,"col":37},"source_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:141:17"},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:145:37","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().local$roomType@145","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":145,"col":37},"source_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:145:17"}]
```

---

### Case 32-34: More Constants

```json
[{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:113:12","kind":"constant","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#BOHO_HOTEL_CODE.","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":113,"col":12}},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:114:12","kind":"constant","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#BOHO_SYNXIS_HOTEL_ID.","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":114,"col":12}},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:117:17","kind":"constant","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#RATE_TYPES.","type":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":117,"col":17}}]
```

---

### Case 35-37: More Method Calls

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:91:58","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer php 8.4.17 DateTime#format().","return_type":"scip-php php builtin . string#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:91:59","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":91,"col":58},"arguments":[{"position":0,"parameter":"scip-php composer php 8.4.17 DateTime#format().($format)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:91:84","value_expr":"\\DateTime::ATOM"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:89:8","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/messenger 51e2b8b6a14b78ad7db60ef5f195ae893c16b9cc Symfony/Component/Messenger/MessageBusInterface#dispatch().","return_type":"scip-php composer symfony/messenger 51e2b8b6a14b78ad7db60ef5f195ae893c16b9cc Symfony/Component/Messenger/Envelope#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:89:15","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":89,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer symfony/messenger 51e2b8b6a14b78ad7db60ef5f195ae893c16b9cc Symfony/Component/Messenger/MessageBusInterface#dispatch().($message)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateCreatedMessage#","value_id":"src/Command/TestSynchronizedItemCommand.php:90:12","value_expr":"$message"},{"position":1,"parameter":"scip-php composer symfony/messenger 51e2b8b6a14b78ad7db60ef5f195ae893c16b9cc Symfony/Component/Messenger/MessageBusInterface#dispatch().($stamps)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:91:12","value_expr":"[new \\App\\Messenger\\Stamp\\EventEnvelopeDataStamp('estate_created', (new \\DateTime())->format(\\DateTime::ATOM), 1)]"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:94:8","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#success().","return_type":"scip-php php builtin . void#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:94:8","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":94,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#success().($message)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:94:21","value_expr":"'Message dispatched! Now consume it:'"}]}]
```

---

### Case 38-40: More Constructor Calls

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:53:21","kind":"constructor","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().","return_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#","receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":53,"col":21},"arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($streetAddressLine1)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:54:36","value_expr":"'123 Test Street'"},{"position":1,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($city)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:55:22","value_expr":"'Test City'"},{"position":2,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($countryCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:56:29","value_expr":"'US'"},{"position":3,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($timezoneCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:57:30","value_expr":"'US-OR'"},{"position":4,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($streetAddressLine2)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:58:36","value_expr":"null"},{"position":5,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($streetAddressLine3)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:59:36","value_expr":"null"},{"position":6,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($zipCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:60:25","value_expr":"'97001'"},{"position":7,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($stateCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:61:27","value_expr":"'OR'"},{"position":8,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($coordinates)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#","value_id":"src/Command/TestSynchronizedItemCommand.php:62:29","value_expr":"new \\App\\Message\\Estate\\EstateAddressCoordinates(latitude: 45.5236, longitude: -122.675)"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:67:21","kind":"constructor","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().","return_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#","receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":67,"col":21},"arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($mainPhone)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:68:27","value_expr":"'5551234567'"},{"position":1,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($secondPhone)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:69:29","value_expr":"'5551234568'"},{"position":2,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($fax)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:70:21","value_expr":"'5551234569'"},{"position":3,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($email)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:71:23","value_expr":"'test@example.com'"},{"position":4,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($url)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:72:21","value_expr":"'https://example.com'"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:74:22","kind":"constructor","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateLanguage#__construct().","return_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateLanguage#","receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":74,"col":22},"arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateLanguage#__construct().($code)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:75:22","value_expr":"'en'"},{"position":1,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateLanguage#__construct().($cultureCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:76:29","value_expr":"'US'"}]}]
```

---

### Case 41-43: More Property Access

```json
[{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:182:15","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#dispatchRoomTypeCreatedEvent().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#$messageBus.","return_type":"scip-php composer symfony/messenger 51e2b8b6a14b78ad7db60ef5f195ae893c16b9cc Symfony/Component/Messenger/MessageBusInterface#","receiver_value_id":null,"location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":182,"col":15},"arguments":[]},{"id":"src/EventListener/MessageValidationFailedEventListener.php:32:15","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/EventListener/MessageValidationFailedEventListener#onMessageValidationFailed().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/EventListener/MessageValidationFailedEventListener#$logger.","return_type":"scip-php composer psr/log f16e1d5863e37f8d8c2a01719f5b34baa2b714d3 Psr/Log/LoggerInterface#","receiver_value_id":null,"location":{"file":"src/EventListener/MessageValidationFailedEventListener.php","line":32,"col":15},"arguments":[]},{"id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:39:35","kind":"access","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#createFromEstateMessage().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateMessage#$contact.","return_type":"scip-php synthetic union . EstateContact|null#","receiver_value_id":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php:39:20","location":{"file":"src/Factory/SynxisHotelCreateRequestPayloadFactory.php","line":39,"col":35},"arguments":[]}]
```

---

### Case 44-46: More Function Calls

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:39:23","kind":"function","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer php 8.4.17 ord().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":39,"col":23},"arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:39:27","value_expr":"$data[8]"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:39:19","kind":"function","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer php 8.4.17 chr().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":39,"col":19},"arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":null,"value_expr":"ord($data[8]) & 0x3f | 0x80"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:40:65","kind":"function","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer php 8.4.17 bin2hex().","return_type":null,"receiver_value_id":null,"location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":40,"col":65},"arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:40:73","value_expr":"$data"}]}]
```

---

### Case 47-49: More Coalesce Operations

```json
[{"id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:85:49","kind":"coalesce","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory#createFromEstateAdditionalFeeMessage().","callee":"scip-php operator . coalesce#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php","line":85,"col":49},"arguments":[],"left_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:85:59","right_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:85:67"},{"id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:86:64","kind":"coalesce","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory#createFromEstateAdditionalFeeMessage().","callee":"scip-php operator . coalesce#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php","line":86,"col":64},"arguments":[],"left_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:86:99","right_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:86:108"},{"id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:105:16","kind":"coalesce","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory#createFromEstateAdditionalFeeMessage().","callee":"scip-php operator . coalesce#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php","line":105,"col":16},"arguments":[],"left_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:105:26","right_id":"src/Factory/SynxisTax/SynxisTaxCreateRequestPayloadFactory.php:105:36"}]
```

---

### Case 50-52: More Array Access

```json
[{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:121:16","kind":"access_array","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().","callee":"","return_type":null,"receiver_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:121:16","location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":121,"col":16},"arguments":[],"key_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:121:26"},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:122:16","kind":"access_array","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().","callee":"","return_type":null,"receiver_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:122:16","location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":122,"col":16},"arguments":[],"key_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:122:26"},{"id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:130:16","kind":"access_array","kind_type":"access","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/DataFixtures/Bottles/SynchronizedItemFixtures#load().","callee":"","return_type":null,"receiver_value_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:130:16","location":{"file":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php","line":130,"col":16},"arguments":[],"key_id":"src/DataFixtures/Bottles/SynchronizedItemFixtures.php:130:26"}]
```

---

### Case 53-55: Calls with Multiple Arguments

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:53:21","kind":"constructor","arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($streetAddressLine1)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:54:36","value_expr":"'123 Test Street'"},{"position":1,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($city)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:55:22","value_expr":"'Test City'"},{"position":2,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($countryCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:56:29","value_expr":"'US'"},{"position":3,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($timezoneCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:57:30","value_expr":"'US-OR'"},{"position":4,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($streetAddressLine2)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:58:36","value_expr":"null"},{"position":5,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($streetAddressLine3)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:59:36","value_expr":"null"},{"position":6,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($zipCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:60:25","value_expr":"'97001'"},{"position":7,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($stateCode)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:61:27","value_expr":"'OR'"},{"position":8,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#__construct().($coordinates)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddressCoordinates#","value_id":"src/Command/TestSynchronizedItemCommand.php:62:29","value_expr":"new \\App\\Message\\Estate\\EstateAddressCoordinates(latitude: 45.5236, longitude: -122.675)"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:67:21","kind":"constructor","arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($mainPhone)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:68:27","value_expr":"'5551234567'"},{"position":1,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($secondPhone)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:69:29","value_expr":"'5551234568'"},{"position":2,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($fax)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:70:21","value_expr":"'5551234569'"},{"position":3,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($email)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:71:23","value_expr":"'test@example.com'"},{"position":4,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#__construct().($url)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:72:21","value_expr":"'https://example.com'"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:44:19","kind":"constructor","arguments":[{"position":0,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:45:22","value_expr":"$estateUuid"},{"position":1,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:46:18","value_expr":"'Test Estate'"},{"position":2,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:47:23","value_expr":"'Test'"},{"position":3,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:48:20","value_expr":"'active'"},{"position":4,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:49:25","value_expr":"'per_property'"},{"position":5,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:50:23","value_expr":"'inactive'"},{"position":6,"parameter":null,"value_type":"scip-php composer php 8.4.17 DateTime#","value_id":"src/Command/TestSynchronizedItemCommand.php:51:23","value_expr":"new \\DateTime()"},{"position":7,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:52:31","value_expr":"'USD'"},{"position":8,"parameter":null,"value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateAddress#","value_id":"src/Command/TestSynchronizedItemCommand.php:53:21","value_expr":"new \\App\\Message\\Estate\\EstateAddress(streetAddressLine1: '123 Test Street', city: 'Test City', countryCode: 'US', timezoneCode: 'US-OR', streetAddressLine2: null, streetAddressLine3: null, zipCode: '97001', stateCode: 'OR', coordinates: new \\App\\Message\\Estate\\EstateAddressCoordinates(latitude: 45.5236, longitude: -122.675))"},{"position":9,"parameter":null,"value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateContact#","value_id":"src/Command/TestSynchronizedItemCommand.php:67:21","value_expr":"new \\App\\Message\\Estate\\EstateContact(mainPhone: '5551234567', secondPhone: '5551234568', fax: '5551234569', email: 'test@example.com', url: 'https://example.com')"},{"position":10,"parameter":null,"value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateLanguage#","value_id":"src/Command/TestSynchronizedItemCommand.php:74:22","value_expr":"new \\App\\Message\\Estate\\EstateLanguage(code: 'en', cultureCode: 'US')"},{"position":11,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:78:21","value_expr":"null"},{"position":12,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:79:41","value_expr":"'notifications@example.com'"},{"position":13,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:80:45","value_expr":"'reservations@example.com'"},{"position":14,"parameter":null,"value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:81:29","value_expr":"true"}]}]
```

---

### Case 56-58: Method Calls with Receiver Chain

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:84:8","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().","return_type":"scip-php php builtin . void#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:84:8","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":84,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().($message)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:84:18","value_expr":"'Dispatching EstateCreatedMessage...'"}]},{"id":"src/Command/TestSynchronizedItemCommand.php:85:8","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().","return_type":"scip-php php builtin . void#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:85:8","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":85,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().($message)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:85:18","value_expr":"\"Estate UUID: {$estateUuid}\""}]},{"id":"src/Command/TestSynchronizedItemCommand.php:86:8","kind":"method","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().","callee":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().","return_type":"scip-php php builtin . void#","receiver_value_id":"src/Command/TestSynchronizedItemCommand.php:86:8","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":86,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#info().($message)","value_type":null,"value_id":"src/Command/TestSynchronizedItemCommand.php:86:18","value_expr":"\"Hotel Code: {$hotelCode}\""}]}]
```

---

### Case 59-61: More Static Methods

```json
[{"id":"src/MessageHandler/EstateCreatedMessageHandler.php:35:8","kind":"method_static","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateCreatedMessageHandler#__construct().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().","return_type":null,"receiver_value_id":null,"location":{"file":"src/MessageHandler/EstateCreatedMessageHandler.php","line":35,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().($synxisHotelCreateRequestPayloadFactory)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#","value_id":"src/MessageHandler/EstateCreatedMessageHandler.php:35:28","value_expr":"$synxisHotelCreateRequestPayloadFactory"},{"position":1,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().($resourceSynxisConnector)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Synxis/Api/Connector/ResourceSynxisConnector#","value_id":"src/MessageHandler/EstateCreatedMessageHandler.php:35:69","value_expr":"$resourceSynxisConnector"},{"position":2,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().($codeProvider)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Service/Synxis/SynxisCodeProvider#","value_id":"src/MessageHandler/EstateCreatedMessageHandler.php:35:95","value_expr":"$codeProvider"},{"position":3,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().($synxisConfigurationService)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Service/Synxis/SynxisConfigurationServiceInterface#","value_id":"src/MessageHandler/EstateCreatedMessageHandler.php:35:110","value_expr":"$synxisConfigurationService"}]},{"id":"src/MessageHandler/EstateUpdatedMessageHandler.php:34:8","kind":"method_static","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateUpdatedMessageHandler#__construct().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().","return_type":null,"receiver_value_id":null,"location":{"file":"src/MessageHandler/EstateUpdatedMessageHandler.php","line":34,"col":8},"arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().($synxisHotelCreateRequestPayloadFactory)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Factory/SynxisHotelCreateRequestPayloadFactory#","value_id":"src/MessageHandler/EstateUpdatedMessageHandler.php:34:28","value_expr":"$synxisHotelCreateRequestPayloadFactory"},{"position":1,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().($resourceSynxisConnector)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Synxis/Api/Connector/ResourceSynxisConnector#","value_id":"src/MessageHandler/EstateUpdatedMessageHandler.php:34:69","value_expr":"$resourceSynxisConnector"},{"position":2,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().($codeProvider)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Service/Synxis/SynxisCodeProvider#","value_id":"src/MessageHandler/EstateUpdatedMessageHandler.php:34:95","value_expr":"$codeProvider"},{"position":3,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateMessageHandler#__construct().($synxisConfigurationService)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Service/Synxis/SynxisConfigurationServiceInterface#","value_id":"src/MessageHandler/EstateUpdatedMessageHandler.php:34:110","value_expr":"$synxisConfigurationService"}]},{"id":"src/MessageHandler/EstateUpdatedMessageHandler.php:76:12","kind":"method_static","kind_type":"invocation","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/MessageHandler/EstateUpdatedMessageHandler#dispatchEstateCreated().","callee":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateCreatedMessage#createFromUpdateMessage().","return_type":null,"receiver_value_id":null,"location":{"file":"src/MessageHandler/EstateUpdatedMessageHandler.php","line":76,"col":12},"arguments":[{"position":0,"parameter":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateCreatedMessage#createFromUpdateMessage().($message)","value_type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateUpdatedMessage#","value_id":"src/MessageHandler/EstateUpdatedMessageHandler.php:76:58","value_expr":"$message"}]}]
```

---

### Case 62-64: More Ternary Full

```json
[{"id":"src/Service/Synxis/SynxisCodeGenerator.php:30:37","kind":"ternary_full","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Service/Synxis/SynxisCodeGenerator#generateCode().","callee":"scip-php operator . ternary#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Service/Synxis/SynxisCodeGenerator.php","line":30,"col":37},"arguments":[],"true_id":"src/Service/Synxis/SynxisCodeGenerator.php:31:18"},{"id":"src/Synxis/Api/SynxisApiErrorHandler.php:47:24","kind":"ternary_full","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Synxis/Api/SynxisApiErrorHandler#handleContentErrors().","callee":"scip-php operator . ternary#","return_type":null,"receiver_value_id":null,"location":{"file":"src/Synxis/Api/SynxisApiErrorHandler.php","line":47,"col":24},"arguments":[],"condition_id":"src/Synxis/Api/SynxisApiErrorHandler.php:47:24","true_id":"src/Synxis/Api/SynxisApiErrorHandler.php:47:35","false_id":"src/Synxis/Api/SynxisApiErrorHandler.php:47:53"},{"id":"tests/Service/SynchronizedItemServiceTest.php:93:24","kind":"ternary_full","kind_type":"operator","caller":"scip-php composer template/u-synxissetup 1.0.0.0 App/Tests/Service/SynchronizedItemServiceTest#testEnsureExistsWorksWithAllItemTypes().","callee":"scip-php operator . ternary#","return_type":null,"receiver_value_id":null,"location":{"file":"tests/Service/SynchronizedItemServiceTest.php","line":93,"col":24},"arguments":[],"true_id":"tests/Service/SynchronizedItemServiceTest.php:93:57","false_id":"tests/Service/SynchronizedItemServiceTest.php:93:64"}]
```

---

### Case 65-67: Local Variables with Types

```json
[{"id":"src/Command/TestSynchronizedItemCommand.php:34:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$io@34","type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":34,"col":8},"source_call_id":"src/Command/TestSynchronizedItemCommand.php:34:14"},{"id":"src/Command/TestSynchronizedItemCommand.php:44:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$message@44","type":"scip-php composer template/u-synxissetup 1.0.0.0 App/Message/Estate/EstateCreatedMessage#","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":44,"col":8},"source_call_id":"src/Command/TestSynchronizedItemCommand.php:44:19"},{"id":"src/Command/TestSynchronizedItemCommand.php:84:8","kind":"local","symbol":"scip-php composer template/u-synxissetup 1.0.0.0 App/Command/TestSynchronizedItemCommand#execute().local$io@34","type":"scip-php composer symfony/console 13d3176cf8ad8ced24202844e9f95af11e2959fc Symfony/Component/Console/Style/SymfonyStyle#","location":{"file":"src/Command/TestSynchronizedItemCommand.php","line":84,"col":8}}]
```

---



## Validation Summary

### Issue 1: Split values and calls ✅

- **Values array**: 4001 entries
  - `local`: 1663
  - `literal`: 1447
  - `parameter`: 581
  - `constant`: 310

- **Calls array**: 3489 entries
  - `method`: 1801
  - `access`: 951
  - `constructor`: 388
  - `function`: 221
  - And more...

### Issue 2: Local variable tracking ✅

- **Unique symbols with @line**: All 1663 local values have `{scope}.local${name}@{line}` format
- **source_call_id tracking**: 476 local values track assignment from calls
- **source_value_id tracking**: 147 local values track assignment from values

### Schema Changes ✅

- `version`: "3.0"
- `kind_type` field on all calls (invocation/access/operator)
- `receiver_value_id` (not `receiver_id`)
- `value_id` in arguments (not `value_call_id`)
- Property access as calls (kind: access, access_static, access_nullsafe)
- Constants as values (kind: constant)

### Evidence Count

Total examples in this file: **67 cases** with **180+ individual entries**

All requirements from `docs/feature-issues/calls-tracking-issues-v3.md` have been implemented and validated.

