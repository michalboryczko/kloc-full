1. pipeline cannot lose data we should somehow has these argument records in graph json.                                                                                                                                                                                                                                
  2. in case of values we should resolve value and attach final type - in case of union types add all options as one of in final - that should be possible because in index.json from sicp-php we should have reference to all of them.                                                                                   
  3. In case of argument resolving
  3.1. remember that we need in graph arguments and reference we have to be able to use that graph for feture like for example check that value has been passed to that method as that argument and then that method used it for something. That is one of critical requirements for flow tracking                        
  3.2. Same for construct arguments so that means we cannot just skip them
  3.3. For literal it can be ok but do we have code snippet? maybe we should keep it and display like `'something'`   
  4. duplication of entries like ```                                                                                                                                                                                                                                                                                      
  [Pasted text #2 +5 lines]                                                                                                                                                                                                                                                                                               
  ``` is redundant data noise. We have `property_access` and then `on: $this->inventoryChecker (App\Service\OrderService::$inventoryChecker)` so that means that call to method is via that property. CRITICAL: To validate if I'm sure we should be able to handle that case if we have some chain of calls we should    
  always handle whole chain as single item in output and as one deep level. We do not want to duplicate these entreis. Handling chain as single thing will allow us in cases like this show allow us to track that simpler                                                                                                
5. missing entries for local variables
6. I think u you forgot about defintion block described in previous things to analyze. 
```php
public function createOrder(CreateOrderInput $input): OrderOutput
    {
        
        $this->inventoryChecker->checkAvailability($input->productId, $input->quantity);
        [1]  App\Component\InventoryCheckerInterface::checkAvailability(string $productId, int $quantity): bool [method_call] (src/Service/OrderService.php:31) 
            on: $this->inventoryChecker (App\Service\OrderService::$inventoryChecker)
            args:
                App\Component\InventoryCheckerInterface::checkAvailability().$productId(string/int whatever)
                    definition:`$input->productId` 
                    value: App\Ui\Rest\Input\CreateOrderInput::$productId(string/int whatever)
                    source: App\Dto\CreateOrderInput::$productId [property_access]
                        on: App\Service\OrderService::createOrder().$input
                        
            // for multi step chain for arguments we should fallow from last chain element to first (for now display all we will handle depth filtration later)
            - ....
        $order = new Order(
            id: 0,
            customerEmail: $input->customerEmail,
            productId: $input->productId,
            quantity: $input->quantity,
            status: 'pending',
            createdAt: new DateTimeImmutable(),
        );
        [1] App\Service\OrderService::createOrder().local#32$order (typr of order) [variable] (src/Service/OrderService.php:32)
                source: App\Entity\Order::__construct(...) [method_call/instntation?] (src/Service/OrderService.php:32)
                args:
                    App\Entity\Order::__construct().$id(int): `0` literal
                
        // Process order through inheritance chain (AbstractOrderProcessor -> StandardOrderProcessor)
        $processedOrder = $this->orderProcessor->process($order);
        [1] App\Service\OrderService::createOrder().local#43$processedOrder (typr of order) [variable/value_assigne(not sure)] (src/Service/OrderService.php:42)
            source: App\Service\AbstractOrderProcessor::process(\App\Entity\Order $order): \App\Entity\Order [method_call] (src/Service/OrderService.php:42)
                    on: $this->orderProcessor (App\Service\OrderService::$orderProcessor)
                    args: 
                        App\Service\AbstractOrderProcessor::process().$order: `$order` App\Service\OrderService::createOrder().local#32$order
                        
```
                                                                                                                                                                                                   
maybe we should consider something like this in case of context

```markdown
1. Definition section
    - What is that method/argument/variable/property/class/interface/etc.?
    - Name of the class/interface/etc. that defines it
    - Dependencies/Schema/not sure about correct name check example 1
2. used by section - it can be as it is now
3. uses section - check example 2
```

example 1
```markdown
Dependencies for:
OrderService::createOrder(CreateOrderInput $input): OrderOutput
Arguments: $input -> CreateOrderInput reference
Return type: OrderOutput -> reference

Dependencies:
- OrderProcessor# class
- Implements in case of any implementation etc or extends
- contains:
    - list of methods references
    - list of properties references value/type
    - list of arguments references
    - list of return types references
```

