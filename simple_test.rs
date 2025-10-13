fn main() {
    // Test PEM parsing logic
    let sample_cert = "-----BEGIN CERTIFICATE-----
MIID3jCCA2WgAwIBAgISLIr3mZ7j2eIbafR6Lk4dbWmiMAoGCCqGSM49BAMDMFox
CzAJBgNVBAYTAlVTMSAwHgYDVQQKExcoU1RBR0lORykgTGV0J3MgRW5jcnlwdDEp
MCcGA1UEAxMgKFNUQUdJTkcpIE15c3RlcmlvdXMgTXVsYmVycnkgRTgwHhcNMjUx
MDAyMTMwOTUzWhcNMjUxMjMxMTMwOTUyWjAZMRcwFQYDVQQDEw5jYS5kYW5zdGVk
Lm9yZzB2MBAGByqGSM49AgEGBSuBBAAiA2IABJbAJyUBmIAhDWeo7hZzd5j0zuFl
j07zPCwHDUMN807v2WWAcyn8EZ5cbgg/URQV4e1oP0W2S/n0VzSK/n/PbiYCg7zA
zEbjAYDZCtJ2IPzIgtqRrbjC3MtBfD6ANdZEL6OCAi0wggIpMA4GA1UdDwEB/wQE
AwIHgDAdBgNVHSUEFjAUBggrBgEFBQcDAQYIKwYBBQUHAwIwDAYDVR0TAQH/BAIw
ADAdBgNVHQ4EFgQUXgUZXJUpMf5+t5NabKiVi0GYPLgwHwYDVR0jBBgwFoAUyUGT
QkjRjBcGkfLyOdKgH6e72zkwNgYIKwYBBQUHAQEEKjAoMCYGCCsGAQUFBzAChhpo
dHRwOi8vc3RnLWU4LmkubGVuY3Iub3JnLzAZBgNVHREEEjAQgg5jYS5kYW5zdGVk
Lm9yZzATBgNVHSAEDDAKMAgGBmeBDAECATAxBgNVHR8EKjAoMCagJKAihiBodHRw
Oi8vc3RnLWU4LmMubGVuY3Iub3JnLzM1LmNybDCCAQ0GCisGAQQB1nkCBAIEgf4E
gfsA+QB/AOT7d0ohJMWGQLGDL1Cr+tyEo4rtcZHutmkiN5LL9iiRAAABmaVArf4A
CAAABQAJgLZcBAMASDBGAiEAy4Vvtl35jRLSrKPW1UsH7NbRN5+KXe7El87tCK9G
cwgCIQDGf/dM4RdzURhh7tuUB3OALrp65KYXDij3745rwiXgcQB2ACh2GhiQJ/vv
PNDWGgGNdrBQVynHp0EbzL32BPRdQmFTAAABmaVAtQwAAAQDAEcwRQIhAIwZTlnw
is0Nk1n02eZ3ojuJNFoOO7T5fYNvgbHlMOxPAiB5kVvQCbxuEoc137kZgYA9YvAX
Yzh6MiVhmy7wufU/YzAKBggqhkjOPQQDAwNnADBkAjBLvSSx3+mS48QmYekQYjOC
9kh/+x4fSIf9QKZC+rO6e4pkKg5sUOS/JVJb0IjZLDoCMDZNFUj3lEg1CRd0UEpy
S12Ca4VIU0S/VlpfFOM8Ptiu3dVhVLcDKT4RKZeAlEVi3g==
-----END CERTIFICATE-----";

    let mut cert_data = String::new();
    let mut in_cert = false;
    
    for line in sample_cert.lines() {
        if line == "-----BEGIN CERTIFICATE-----" {
            in_cert = true;
            cert_data = String::new();
        } else if line == "-----END CERTIFICATE-----" {
            in_cert = false;
            if !cert_data.is_empty() {
                println!("✅ Found certificate data, length: {}", cert_data.len());
                println!("First 50 chars: {}", &cert_data[..50.min(cert_data.len())]);
                println!("Certificate parsing test PASSED!");
                return;
            }
        } else if in_cert {
            cert_data.push_str(line);
        }
    }
    
    println!("❌ No certificate found in sample data");
}

