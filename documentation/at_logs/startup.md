--------------- Initialization start ---------------

<!-- Check that modem responds -->
>> "AT"
<< OK

<!-- Check SIM Pin code -->
>> "AT+CPIN?"
<< response: "+CPIN: READY"

<!-- Disable verbose logging (ATAT doesn't support verbose) -->
>> "AT+CMEE=0"
<< OK

<!-- Setup DCD pin behaviour -->
>> "AT&C1"
<< OK

<!-- Ignore DTR pin behaviour -->
>> "AT&D0"
<< OK

<!-- Switch off UART power saving -->
>> "AT+UPSV=0"
<< OK

<!-- Set data communication in HEX mode -->
>> "AT+UDCONF=1,1"
<< OK

<!-- Enable RTS/CTS Flow control -->
>> "AT&K3"
<< OK

<!-- Disable "Message waiting indications" (voice-mail) -->
>> "AT+UMWI=0"
<< OK

<!-- Put the module in full mode -->
>> "AT+CFUN=1"
<< OK

<!-- Discard Packet domain event reporting for now -->
>> "AT+CGEREP=1"
<< OK

<!-- Enable 2G/3G/4G registration events -->
>> "AT+CREG=2"
<< OK

<!-- Enable 2G/3G registration events -->
>> "AT+CGREG=2"
<< OK

<!-- Enable 4G registration events -->
>> "AT+CEREG=2"
<< OK

<!-- Check operator selection -->
>> "AT+COPS?"
<< "+COPS: 0,0,"3 DK",2"

<!-- Check 2G/3G/4G registration -->
>> "AT+CREG?"
<< "+CREG: 2,5,"9E9A","019607C0",2"         (2 = URC enabled, 5 = RegisteredRoaming, 9E9A = AreaCode, 019607C0 = CellId, 2 = 3G)

<!-- Check 2G/3G registration -->
>> "AT+CGREG?"
<< "+CGREG: 2,5,"9E9A","019607C0",2,"02""   (2 = URC enabled, 5 = RegisteredRoaming, 9E9A = AreaCode, 019607C0 = CellId, 2 = 3G, 02 = RoutingArea)

<!-- Check 4G registration -->
>> "AT+CEREG?"
<< "+CEREG: 2,0"                            (2 = URC enabled, 0 = NotRegistered)

--------------- Initialization finished ---------------

<!-- Set the module in minimal mode -->
>> "AT+CFUN=0"
<< OK

<!-- Setup APN settings for Emnify -->
>> "AT+CGDCONT=1,"IP","em""
<< OK

<<< +CGEV: ME DETACH
<<< +CREG: 0
<<< +CGREG: 0
<<< +CEREG: 0

<!-- Setup network authentication (no username/password) -->
>> "AT+UAUTHREQ=1,3,"","""
<< OK

<!-- Put the module in full mode -->
>> "AT+CFUN=1"
<< OK

<<< +CGEV: ME CLASS "B"
<<< +CREG: 5,"9E9A","0196BDB0",2
<<< +CGREG: 2
<<< +CEREG: 4
<<< +CGEV: NW CLASS "A"
<<< +CGEV: NW CLASS "A"
<<< +CGREG: 5,"9E9A","0196BDB0",2,"02"
<<< +CEREG: 4

<!-- Check 2G/3G/4G registration -->
>> "AT+CREG?"
<< "+CREG: 2,5,"9E9A","0196BDB0",2"

<!-- Check 2G/3G registration -->
>> "AT+CGREG?"
<< "+CGREG: 2,5,"9E9A","0196BDB0",2,"02""

<!-- Check 4G registration -->
>> "AT+CEREG?"
<< "+CEREG: 2,0"

<!-- Check PDP Context activation -->
>> "AT+CGACT?"
<< "+CGACT: 1,0"

<!-- Activate PDP Context -->
>> "AT+CGACT=1,1"
<< OK

<!-- Link internal IP stack to PDP context -->
>> "AT+UPSD=0,100,1"
<< OK

<<< +CGEV: ME PDN ACT 1

<!-- Activate internal IP stack -->
>> "AT+UPSDA=0,3"
<< OK

--------------- Connected! ---------------

<!-- Create UDP socket -->
>> "AT+USOCR=17"                            (17 = UDP)
<< "+USOCR: 0"

<<< +UUPSDA: 0,"100.92.188.77"

<!-- Connect UDP socket -->
>> "AT+USOST=0,"185.15.72.251",123,48"
<< OK
