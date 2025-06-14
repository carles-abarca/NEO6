#!/bin/bash

# Script de prueba para verificar que la página de detalle del proxy muestre los protocolos

echo "=== Prueba de la página de detalle del proxy ==="
echo ""

# 1. Verificar que la API devuelve datos
echo "1. Verificando datos de la API:"
API_RESPONSE=$(curl -s http://localhost:8090/api/proxies/tn3270-primary)
echo "Status: $(echo $API_RESPONSE | grep -o '"status":"[^"]*"')"
echo "Protocols loaded: $(echo $API_RESPONSE | grep -o '"protocols_loaded":\[[^]]*\]')"
echo "Proxy responsive: $(echo $API_RESPONSE | grep -o '"proxy_responsive":[^,}]*')"
echo ""

# 2. Verificar que la página HTML carga
echo "2. Verificando que la página HTML carga:"
HTML_RESPONSE=$(curl -s http://localhost:8090/proxy/tn3270-primary)
if echo "$HTML_RESPONSE" | grep -q "Proxy: tn3270-primary"; then
    echo "✓ Página HTML carga correctamente"
else
    echo "✗ Error cargando página HTML"
fi

# 3. Verificar que el logo se está sirviendo
echo ""
echo "3. Verificando acceso al logo:"
LOGO_STATUS=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8090/static/neo6.png)
if [ "$LOGO_STATUS" = "200" ]; then
    echo "✓ Logo accesible en /static/neo6.png"
else
    echo "✗ Error accediendo al logo (HTTP $LOGO_STATUS)"
fi

echo ""
echo "=== Fin de la prueba ==="
echo ""
echo "Para verificar visualmente:"
echo "- Dashboard: http://localhost:8090"
echo "- Detalle del proxy: http://localhost:8090/proxy/tn3270-primary"
echo "- Logo directo: http://localhost:8090/static/neo6.png"
