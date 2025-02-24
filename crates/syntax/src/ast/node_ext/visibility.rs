use crate::ast;

impl ast::Visibility {
    pub fn is_public(&self) -> bool {
        self.public_token().is_some() && self.l_paren_token().is_none()
    }

    pub fn is_public_script(&self) -> bool {
        self.public_token().is_some()
            && self.l_paren_token().is_some()
            && self.script_token().is_some()
            && self.l_paren_token().is_some()
    }

    pub fn is_public_friend(&self) -> bool {
        self.public_token().is_some()
            && self.l_paren_token().is_some()
            && self.friend_token().is_some()
            && self.l_paren_token().is_some()
    }

    pub fn is_public_package(&self) -> bool {
        self.public_token().is_some()
            && self.l_paren_token().is_some()
            && self.package_token().is_some()
            && self.l_paren_token().is_some()
    }
}
