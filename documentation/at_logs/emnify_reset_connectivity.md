<<< +CGEV: ME DETACH
<<< +CGREG: 0

<!-- Connection state changed to "Connecting" -->

<<< +CEREG: 0
<<< +CGEV: NW CLASS "A"
<<< +CGEV: ME PDN DEACT 1
<<< +UUPSDD: 0
<<< +UUSOCL: 0
<<< +CGEV: NW CLASS "A"

>> "AT+CREG?"
<< "+CREG: 2,5,"9E9A","019624BD",2"

>> "AT+CGREG?"
<< "+CGREG: 2,0"

>> "AT+CEREG?"
<< "+CEREG: 2,0"

>> "AT+CREG?"
<< "+CREG: 2,5,"9E9A","019624BD",2"

>> "AT+CGREG?"
<< "+CGREG: 2,0"

>> "AT+CEREG?"
<< "+CEREG: 2,0"

>> "AT+CGACT?"
<< "+CGACT: 1,0"

>> "AT+CGACT=1"
<< OK

<<< +CGREG: 5,"9E9A","019624BD",2,"02"

<!-- Connection state changed to "Connected" -->

>> "AT+CGACT?"
<< "+CGACT: 1,1"

<<< +CEREG: 4
<<< +CGEV: NW CLASS "A"
<<< +CGEV: ME PDN ACT 1

>> "AT+UPSD=0,100,1"
<< OK

>> "AT+UPSDA=0,3"
<< OK

<<< +UUPSDA: 0,"100.92.188.77"

>> "AT+USOCR=6"
<< "+USOCR: 0"

