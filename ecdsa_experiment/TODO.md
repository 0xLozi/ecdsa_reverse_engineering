**ECC Engine Testing Strategy**
Do not unit test intermediate functions like point_double directly because they output values in Montgomery form. Instead, test the full public key derivation function (ec_priv_to_pub) which properly converts coordinates back to standard form at the end.

<!-- CREATE ATOMIC TESTS FOR THESE FUNCTIONS IN ORDER TO SEE WHICH ONES DIFFER FROM THE RESULTS FROM C:

point_double
├── mod_square ✓ checked and passed
├── mod_add 
├── mod_inv
├── mod_mul
└── mod_sub

So once all 5 passes and their results are the same with the corresponding C results. Then test point_double as a hole and compare. -->

