//! Pruebas unitarias para todos los protocolos mockeados en neo6-protocols

#[cfg(test)]
mod tests {
    use neo6_protocols_lib::protocol::ProtocolHandler;
    use serde_json::json;

    #[test]
    fn test_tcp_handler() {
        let handler = tcp::TcpHandler;
        let params = json!({"foo": "bar"});
        let result = handler.invoke_transaction("TXTCP", params.clone()).unwrap();
        assert_eq!(result["protocol"], "tcp");
        assert_eq!(result["transaction_id"], "TXTCP");
        assert_eq!(result["parameters"], params);
    }

    #[test]
    fn test_lu62_handler() {
        let handler = lu62::Lu62Handler;
        let params = json!({"foo": "bar"});
        let result = handler.invoke_transaction("TXLU62", params.clone()).unwrap();
        assert_eq!(result["protocol"], "lu62");
        assert_eq!(result["transaction_id"], "TXLU62");
        assert_eq!(result["parameters"], params);
    }

    #[test]
    fn test_mq_handler() {
        let handler = mq::MqHandler;
        let params = json!({"foo": "bar"});
        let result = handler.invoke_transaction("TXMQ", params.clone()).unwrap();
        assert_eq!(result["protocol"], "mq");
        assert_eq!(result["transaction_id"], "TXMQ");
        assert_eq!(result["parameters"], params);
    }

    #[test]
    fn test_rest_handler() {
        let handler = rest::RestHandler;
        let params = json!({"foo": "bar"});
        let result = handler.invoke_transaction("TXREST", params.clone()).unwrap();
        assert_eq!(result["protocol"], "rest");
        assert_eq!(result["transaction_id"], "TXREST");
        assert_eq!(result["parameters"], params);
    }

    #[test]
    fn test_jca_handler() {
        let handler = jca::JcaHandler;
        let params = json!({"foo": "bar"});
        let result = handler.invoke_transaction("TXJCA", params.clone()).unwrap();
        assert_eq!(result["protocol"], "jca");
        assert_eq!(result["transaction_id"], "TXJCA");
        assert_eq!(result["parameters"], params);
    }
}
